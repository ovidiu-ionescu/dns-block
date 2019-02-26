use memmap::MmapOptions;
use std::fs::File;
use std::str;

fn main() {
  let file = File::open("domains.blocked").expect("Failed to open file");
  let mmap = unsafe { MmapOptions::new().map(&file).expect("Failed to memmap file") };
  
  // assert_eq!(b"14933616", &mmap[0..8]);

  let entire_file = unsafe { str::from_utf8_unchecked(&mmap) };

  #[derive(Debug,PartialEq)]
  enum LinePosition {
    LineStart,
    LineEnd
  }

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
  println!("Stats: characters {}, lines {}", count, line_count);


  // println!("From file {}", &tot_fisierul[1..10]);
  // println!("Hello, world!");
}
