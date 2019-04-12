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


  let domain_block_string = fs::read_to_string(domain_block_filename).unwrap();
  let hosts_blocked_string = fs::read_to_string(hosts_blocked_filename).unwrap();

  let mut blacklist: HashSet<&str> = HashSet::default();


  // domains to blacklist should be processed from shortest
  // to longest

  let start_sorting = start.elapsed().as_millis();
  // println!("calculate max number of lines");
  let total = count_char_occurences(&domain_block_string, '\n')
    + count_char_occurences(&hosts_blocked_string, '\n');
  // println!("allocate a vector to fit all {} lines", total);
  let mut bad_domains: Vec<Domain> = Vec::with_capacity(total);

  // println!("put all lines in the vector");
  for line in hosts_blocked_string.lines() {
    if let Some(s) = line.split_whitespace().next() {
      if !ignore_line(s) {
        bad_domains.push(Domain::new(s));
      }
    }
  }

  for line in domain_block_string.lines() {
    if let Some(s) = line.split_whitespace().next() {
      if !ignore_line(s) {
        bad_domains.push(Domain::new(s));
      }
    }
  }
  // println!("sort the vector, less dots first");
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
  // println!("add all baddies to the index");
  for domain in &bad_domains {
    process_bad_domain(domain.name, &mut blacklist, &whitelist);
  }
  // println!("Done processing");

  // for line in hosts_blocked_string.lines() {
  //   process_line(line, &mut blacklist, &whitelist);
  // }

  // for line in domain_block_string.lines() {
  //   process_line(line, &mut blacklist, &whitelist);
  // }
  let start_writing = start.elapsed().as_millis();
  write_output(&blacklist);

  println!("sorting: {}, until after sort: {}, processing baddies: {}", 
    end_sorting - start_sorting, start_baddies, start_writing - start_baddies);
}

/// Adds a non comment line to the whitelist index
/// Optionally it can add the domain to the list with the actual domains
fn process_whitelist_line<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
  if let Some(s) = line.split_whitespace().next() {
    if !ignore_line(s) {
      for seg in sub_domains::SubDomains::new(s, 0) {
        index.insert(seg);
      }
    }
  }
}

fn process_bad_domain<'a>(
  domain: &'a str,
  index: &mut HashSet<&'a str>,
  whitelist: &HashSet<&'a str>,
) {
  let mut seg_num = 0;
  for seg in sub_domains::SubDomains::new(domain, 1) {
    if index.contains(seg) {
      return;
    }
    seg_num += 1;
  }
  if seg_num > 1 && !whitelist.contains(domain) {
    index.insert(domain);
  }
}

fn write_output(index: & HashSet<&str>) {
  let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create("simple.blocked").unwrap());
  let eol: [u8; 1] = [10];
  for d in index.iter() {
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

