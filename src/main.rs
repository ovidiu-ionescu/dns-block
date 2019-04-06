use std::env;

use std::collections::HashSet;

use std::fs;
use std::io::{BufWriter, Write};

mod sub_domains;
mod dns_resolver;

fn main() {
  let args: Vec<String> = env::args().collect();
  let domain_block_filename = &args[1];
  let whitelist_filename = &args[2];
  let hosts_blocked_filename = &args[3];

  let domain_block_string = fs::read_to_string(domain_block_filename).unwrap();
  let whitelist_string = fs::read_to_string(whitelist_filename).unwrap();
  let hosts_blocked_string = fs::read_to_string(hosts_blocked_filename).unwrap();

  let mut blacklist: HashSet<&str> = HashSet::new();
  let mut whitelist: HashSet<&str> = HashSet::new();

  let mut active_whitelist = Vec::with_capacity(50);

  for line in whitelist_string.lines() {
    process_whitelist_line(line, &mut whitelist, Some(&mut active_whitelist));
  }

  let mut extra_whitelist = Vec::new();

  // fetch the other domains to whitelist
  dns_resolver::resolve_domain(&active_whitelist, &mut extra_whitelist);
  for line in &extra_whitelist {
    let domain = &line[.. line.len() - 1];
    process_whitelist_line(domain, &mut whitelist, None);
  }

  for line in hosts_blocked_string.lines() {
    process_line(line, &mut blacklist, &whitelist);
  }

  for line in domain_block_string.lines() {
    process_line(line, &mut blacklist, &whitelist);
  }

  write_output(&blacklist);
}

/// Adds a non comment line to the whitelist index
/// Optionally it can add the domain to the list with the actual domains
fn process_whitelist_line<'a>(line: &'a str, index: &mut HashSet<&'a str>, non_comment_lines: Option<&mut Vec<&'a str>>) {
  if let Some(s) = line.split_whitespace().next() {
    if !ignore_line(s) {
      for seg in sub_domains::SubDomains::new(s) {
        index.insert(seg);
      }
      if let Some(list) = non_comment_lines {
        list.push(s);
      }
    }
  }
}

fn process_line<'a>(line: &'a str, index: &mut HashSet<&'a str>, whitelist: &HashSet<&'a str>) {
  if let Some(s) = line.split_whitespace().next() {
    let mut seg_num = 0;
    for seg in sub_domains::SubDomains::new(s) {
        if index.contains(seg) {
          return;
        }
        seg_num += 1;
    }
    if seg_num > 1 && !whitelist.contains(s) { 
      index.insert(s); 
    }
  }
}

fn write_output(index: &HashSet<&str>) {
  let mut f = BufWriter::new(fs::File::create("simple.blocked").unwrap());
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
    return true
  }
  if '#' == line.chars().next().unwrap() {
    return true
  }
  false
}