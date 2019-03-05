use memmap::MmapOptions;
use std::fs::File;
use std::str;
use std::env;

use std::collections::HashMap;

  #[derive(Debug,PartialEq)]
  enum LinePosition {
    LineStart,
    LineEnd
  }

fn main() {
  let args: Vec<String> = env::args().collect();
  let filename = &args[1];

  let file = File::open(filename).expect("Failed to open file");
  let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memmap file") };

  let entire_file = unsafe { str::from_utf8_unchecked(&mmap) };


  let mut status = LinePosition::LineEnd;
  let mut start = 0;
  let mut end = 0;
  let mut count = 0;
  let mut line_count = 0;
  for (i, c) in entire_file.char_indices() {
    count+=1;
    match c {
      '\n' | '\r' => {
        if let LinePosition::LineStart = status {
          status = LinePosition::LineEnd;
          if i > start + 1 {
            end = i;
            process_line(&entire_file[start..end]);
          }
        }
      },
      _ => {
        if LinePosition::LineEnd == status {
          status = LinePosition::LineStart;
          start = i;
          line_count += 1;
        }
      }
    }
  }
  if end < start {
    process_line(&entire_file[start..]);
  }
  println!("Stats: characters {}, lines {}", count, line_count);


  // println!("From file {}", &tot_fisierul[1..10]);
  // println!("Hello, world!");
}

fn process_line(line: &str) {
  for item in line.split('.').rev() {
    print!("{};", item);
  }
  println!(" = {}", line);
}

struct HashToHash<'a>(HashMap<&'a str, HashToHash<'a>>);

struct DomainIndex<'a> {
  map: HashToHash<'a>
}