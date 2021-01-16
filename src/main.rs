use std::env;

//use std::collections::HashSet;
use fnv::FnvHashSet as HashSet;

use std::fs;
use std::io::{BufWriter, Write};

use std::thread;
use std::sync::mpsc;

mod dns_resolver;
mod sub_domains;
use sub_domains::{count_char_occurences, Domain, sub_domain_iterator};
mod filter;
mod statistics;
use statistics::Statistics;

use std::time::{Instant};

use rayon::join;

use clap::{clap_app, crate_version};
use log::*;
use indoc::indoc;

fn main() {
  let command_line_params = clap_app!(
    ("dns-block") => 
    (version: crate_version!())
    (author: "Ovidiu Ionescu <ovidiu@ionescu.net>")
    (about: "Simplify the list of ad and tracker servers")
    (@arg debug: -d +multiple "Set debug level debug information")
    (@arg filter: -f --filter "act as filter on stdin")
    (@arg bind: -b --bind "output in Bind format")
    (@arg ("domains.blocked"): +required "File containing the list of servers to block")
    (@arg ("domains.whitelisted"): +required "File containing the list of servers to whitelist")
    (@arg ("hosts_blocked.txt"): +required "Additional personal file with domains to block")
    (@arg ("output_file"): default_value("simple.blocked") "Output file")
).get_matches();

  let log_level = command_line_params.occurrences_of("debug") as usize;
  stderrlog::new()
  .module(module_path!())
  .quiet(false)
  .verbosity(log_level)
  .timestamp(stderrlog::Timestamp::Off)
  .init()
  .unwrap();

  trace!("{:#?}", command_line_params);

  let start = Instant::now();

  let domain_block_filename = command_line_params.value_of("domains.blocked").unwrap();
  let whitelist_filename = command_line_params.value_of("domains.whitelisted").unwrap();
  let hosts_blocked_filename = command_line_params.value_of("hosts_blocked.txt").unwrap();
  let output_file = command_line_params.value_of("output_file").unwrap();

  let whitelist_string = fs::read_to_string(whitelist_filename).unwrap();

  // do the DNS requests while we read and sort the domains to block
  let(tx, rx) = mpsc::channel();
  thread::spawn(move || {
    tx.send(expand_whitelist(whitelist_string)).unwrap();
  });


  let mut domain_block_string = fs::read_to_string(domain_block_filename).unwrap();
  let hosts_blocked_string = fs::read_to_string(hosts_blocked_filename).unwrap();

  // converting to lowercase might generate some duplicates
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

  for domain in &cnames {
    process_whitelist_line(domain, &mut whitelist);
  }

  let start_baddies = start.elapsed().as_millis();

  let ((blacklist_com, statistics_com), (blacklist_net, statistics_net)) = join (
    || process_baddies(&bad_domains, &whitelist, |s: &str| s.ends_with("com")),
    || process_baddies(&bad_domains, &whitelist, |s: &str| !s.ends_with("com"))
  ); 
  info!("Statistics .com \n{}", &statistics_com);
  info!("Statistics .net \n{}", &statistics_net);
  info!("Statistics total \n{}", Statistics::aggregate(&statistics_com, &statistics_net));

  if command_line_params.is_present("filter") {
    filter::filter(&blacklist_com, &blacklist_net).unwrap();
  } else {
    let start_writing = start.elapsed().as_millis();
    if command_line_params.is_present("bind") {
      write_bind_output(&blacklist_com, &blacklist_net, output_file);
    } else {
      write_output(&blacklist_com, &blacklist_net, output_file);
    }

    debug!("sorting: {}, sorting core: {}, until after sort: {}, processing baddies: {}", 
      end_sorting - start_sorting, end_sorting - start_sorting_code, start_baddies, start_writing - start_baddies);
  }
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

/// adds a domain to the blocked index if it's not already blocked already or whitelisted
fn process_bad_domain<'a>(
  domain: &'a str,
  index: &mut HashSet<&'a str>,
  whitelist: &HashSet<&'a str>,
  statistics: &mut Statistics,
) {
  if domain.is_empty() {
    return;
  }
  for seg in sub_domain_iterator(domain, 1) {
    if index.contains(seg) {
      statistics.increment_parent();
      return;
    }
  }
  if !whitelist.contains(domain) {
    if index.insert(domain) {
      statistics.increment_blocked();
    } else {
      statistics.increment_duplicate();
    }
  } else {
    statistics.increment_whitelisted();
  }
}

fn write_output(index_com: & HashSet<&str>, index_net: & HashSet<&str>, output_file: &str) {
  let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create(output_file).unwrap());
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

fn write_bind_output(index_com: & HashSet<&str>, index_net: & HashSet<&str>, output_file: &str) {
  let preamble = indoc! {"
    $TTL 60
    @   IN    SOA  localhost. root.localhost.  (
            2   ; serial 
            3H  ; refresh 
            1H  ; retry 
            1W  ; expiry 
            1H) ; minimum 
        IN    NS    localhost.
    "};
  let prefix = "*.";
  let suffix = " CNAME .";
  let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create(output_file).unwrap());
  
  f.write(&preamble.as_bytes()).unwrap();

  let eol: [u8; 1] = [10];
  let mut serialize_index = | index: &HashSet<&str> | {
    for d in index.iter() {
      f.write(&*d.as_bytes()).unwrap();
      f.write(&suffix.as_bytes()).unwrap();
      f.write(&eol).unwrap();

      f.write(&prefix.as_bytes()).unwrap();
      f.write(&*d.as_bytes()).unwrap();
      f.write(&suffix.as_bytes()).unwrap();
      f.write(&eol).unwrap();
    }
  };
  serialize_index(index_com);
  serialize_index(index_net);

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
  dns_resolver::resolve_domain(&explicit_whitelisted_domains, &mut cnames).unwrap();
  debug!("Cnames to be whitelisted: {:#?}", cnames);
  (whitelist_string, cnames)
}

/// Makes an index from a list of domains to block
/// filter selects a subset of domains to process, e.g. .com ones
fn process_baddies<'a>(
  bad_domains: &'a Vec<Domain>, 
  whitelist: &HashSet<&'a str>, 
  filter_d: fn(&str) -> bool) -> (HashSet<&'a str>, Statistics) {

  let mut blacklist: HashSet<&str> = HashSet::with_capacity_and_hasher(bad_domains.len() / 2, Default::default());
  let mut statistics = Statistics::new();

  for domain in bad_domains.iter().filter(|d| filter_d(d.name)) {
    process_bad_domain(domain.name, &mut blacklist, &whitelist, &mut statistics);
  }
  (blacklist, statistics)
}
