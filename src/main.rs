use memmap::MmapOptions;
use std::fs::File;
use std::str;
use std::env;

use std::collections::HashMap;
use std::collections::HashSet;

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

  let file = File::open(filename).expect("Failed to open file");
  let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memmap file") };

  let entire_file = unsafe { str::from_utf8_unchecked(&mmap) };

  let mut domain_index = HashToHash(HashMap::new());

  let mut blacklist: HashSet<&str> = HashSet::new();

  for line in entire_file.lines() {
    process_line(line, &mut blacklist);
    //add_domain(&mut domain_index, line);

  }

}

fn _process_line(line: &str) {
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

fn process_line<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
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
