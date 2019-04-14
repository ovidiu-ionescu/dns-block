use std::env;

//use std::collections::HashSet;
use fnv::FnvHashSet as HashSet;

use std::fs;
use std::io::{BufWriter, Write};

use std::thread;
use std::sync::mpsc;

mod dns_resolver;
mod sub_domains;
use sub_domains::{count_char_occurences, Domain};

use std::time::{Instant};

use rayon::join;

fn main() {
  let start = Instant::now();

  let args: Vec<String> = env::args().collect();
  let domain_block_filename = &args[1];
  let whitelist_filename = &args[2];
  let hosts_blocked_filename = &args[3];

  let whitelist_string = fs::read_to_string(whitelist_filename).unwrap();

  // do the DNS requests while we read and sort the domains to block
  let(tx, rx) = mpsc::channel();
  thread::spawn(move || {
    tx.send(expand_whitelist(whitelist_string)).unwrap();
  });


  let mut domain_block_string = fs::read_to_string(domain_block_filename).unwrap();
  let hosts_blocked_string = fs::read_to_string(hosts_blocked_filename).unwrap();

  
  domain_block_string.make_ascii_lowercase();

  // domains to blacklist should be processed from shortest
  // to longest

  let start_sorting = start.elapsed().as_millis();
  // println!("calculate max number of lines");
  let total = count_char_occurences(&domain_block_string, '\n')
    + count_char_occurences(&hosts_blocked_string, '\n');

  // println!("allocate a vector to fit all {} lines", total);
  let mut bad_domains: Vec<Domain> = Vec::with_capacity(total);

  // println!("put all lines from the personal block list in the vector");
  for line in hosts_blocked_string.lines() {
    if let Some(s) = line.split_whitespace().next() {
      if !ignore_line(s) {
        let domain = Domain::new(s);
        bad_domains.push(domain);
      }
    }
  }

  // println!("put all lines from the public block list in the vector");
  for line in domain_block_string.lines() {
    if let Some(s) = line.split_whitespace().next().as_mut() {
      if !ignore_line(s) {
        let domain = Domain::new(s);
        if domain.dots > 0 {
          bad_domains.push(domain);
        }
      }
    }
  }
  // println!("sort the vector, less dots first");
  let start_sorting_code = start.elapsed().as_millis();
  bad_domains.sort_unstable_by_key(|d: &Domain| d.dots);
  let end_sorting = start.elapsed().as_millis();

  // Prepare the whitelist index
  // get the cnames from the other thread
  let (whitelist_string, cnames) = rx.recv().unwrap();

  let mut whitelist: HashSet<&str> = HashSet::default();

  for line in whitelist_string.lines() {
    process_whitelist_line(line, &mut whitelist);
  }

  for line in &cnames {
    let domain = &line[..line.len() - 1];
    process_whitelist_line(domain, &mut whitelist);
  }

  let start_baddies = start.elapsed().as_millis();

  let (blacklist_com, blacklist_net) = join(
    || process_baddies(&bad_domains, &whitelist, |s: &str| s.ends_with("com")),
    || process_baddies(&bad_domains, &whitelist, |s: &str| !s.ends_with("com"))
  ); 

  let start_writing = start.elapsed().as_millis();
  write_output(&blacklist_com, &blacklist_net);

  println!("sorting: {}, sorting core: {}, until after sort: {}, processing baddies: {}", 
    end_sorting - start_sorting, end_sorting - start_sorting_code, start_baddies, start_writing - start_baddies);
}

/// Adds a non comment line to the whitelist index
/// Optionally it can add the domain to the list with the actual domains
fn process_whitelist_line<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
  if let Some(s) = line.split_whitespace().next() {
    if !ignore_line(s) {
      for seg in sub_domain_iterator(s, 1) {
        index.insert(seg);
      }
      index.insert(s);
    }
  }
}

fn process_bad_domain<'a>(
  domain: &'a str,
  index: &mut HashSet<&'a str>,
  whitelist: &HashSet<&'a str>,
) {
  if domain.is_empty() {
    return;
  }
  for seg in sub_domain_iterator(domain, 1) {
    
    if index.contains(seg) {
      return;
    }
  }
  if !whitelist.contains(domain) {
    index.insert(domain);
  }
}

fn write_output(index_com: & HashSet<&str>, index_net: & HashSet<&str>) {
  let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create("simple.blocked").unwrap());
  let eol: [u8; 1] = [10];
  for d in index_com.iter() {
    f.write(&*d.as_bytes()).unwrap();
    f.write(&eol).unwrap();
  }
  for d in index_net.iter() {
    f.write(&*d.as_bytes()).unwrap();
    f.write(&eol).unwrap();
  }
  f.flush().unwrap();
}

/// Checks if a line is empty or a comment
fn ignore_line(line: &str) -> bool {
  if line.is_empty() {
    return true;
  }
  if '#' == line.chars().next().unwrap() {
    return true;
  }
  false
}

// expand the whitelisted domains with their cnames
fn expand_whitelist(whitelist_string: String) -> (String, Vec<String>) {
  // println!("fetch the other domains to whitelist");

  let mut explicit_whitelisted_domains = Vec::with_capacity(50);
  for line in whitelist_string.lines() {
    if let Some(s) = line.split_whitespace().next() {
      if !ignore_line(s) {
          explicit_whitelisted_domains.push(s);
        }
    }
  }
  let mut cnames = Vec::with_capacity(50);
  dns_resolver::resolve_domain(&explicit_whitelisted_domains, &mut cnames);
  (whitelist_string, cnames)
}

fn sub_domain_iterator<'a>(domain: &'a str, min: usize) -> impl Iterator<Item = &'a str> {
  domain.char_indices().rev()
    .filter(|(_i, c)| *c == '.')
    .skip(min)
    .map(move |(i, _c)| &domain[i + 1 ..])
}

#[test]
fn sub_domain_iterator_test() {
  let mut subdomains = sub_domain_iterator("many.ads.fb.com", 1);
  assert_eq!("fb.com", subdomains.next().unwrap());
  assert_eq!("ads.fb.com", subdomains.next().unwrap());
  assert_eq!(None, subdomains.next());
}

fn process_baddies<'a>(bad_domains: &'a Vec<Domain>, whitelist: &HashSet<&'a str>, filter_d: fn(&str) -> bool) ->HashSet<&'a str> {
    let mut blacklist: HashSet<&str> = HashSet::with_capacity_and_hasher(bad_domains.len() / 2, Default::default());

  for domain in bad_domains.iter().filter(|d| filter_d(d.name)) {
    process_bad_domain(domain.name, &mut blacklist, &whitelist);
  }
  blacklist
}