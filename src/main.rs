use memmap::MmapOptions;
use std::fs::File;
use std::str;
use std::env;

use std::collections::HashMap;
use std::collections::HashSet;

use std::fs;
use std::io::{BufWriter, Write};

mod sub_domains;

  #[derive(Debug,PartialEq)]
  enum LinePosition {
    LineStart,
    LineEnd
  }

struct HashToHash<'a>(HashMap<&'a str, HashToHash<'a>>);

fn main() {
  let args: Vec<String> = env::args().collect();
  let filename = &args[1];

  // let file = File::open(filename).expect("Failed to open file");
  // let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memmap file") };

  // let domain_block_string = unsafe { str::from_utf8_unchecked(&mmap) };

  let domain_block_string = fs::read_to_string(&args[1]).unwrap();

  let mut domain_index = HashToHash(HashMap::new());

  let mut blacklist: HashSet<&str> = HashSet::new();

  for line in domain_block_string.lines() {
    process_line(line, &mut blacklist);
    //add_domain(&mut domain_index, line);

  }

  write_output(&blacklist);
}

fn process_line1(line: &str) {
  if line.len() < 1 {
    return
  }
  for item in line.split('.').rev() {
    print!("{};", item);
  }
  println!(" = {}", line);
}

fn add_domain<'b>(domain_index: &mut HashToHash<'b>, line: &'b str) {
  if line.len() < 1 {
    return
  }

  let mut index = domain_index;
  for item in line.split('.').rev() {
    print!("{};", item);

    index = index.0.entry(item).or_insert(HashToHash(HashMap::new()));

  }
  println!(" = {}", line);
}

fn process_line2<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
  if let Some(s) = line.split_whitespace().next() {
    for (i, c) in s.char_indices().rev() {
      if '.' == c || 0 == i {
        let seg;
        if '.' == c {
          seg = &s[i + 1..];
        } else {
          seg = s;
        }
        if !index.contains(seg) {
          index.insert(s);
          break;
        }
      }
    }
  }
}

fn process_line<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
  if let Some(s) = line.split_whitespace().next() {
    let mut seg_num = 0;
    for seg in sub_domains::SubDomains::new(s) {
        if index.contains(seg) {
          return;
        }
        seg_num += 1;
    }
    if seg_num > 1 { index.insert(s); }
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