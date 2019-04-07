
pub struct SubDomains<'a> {
  front_offset: usize,
  iter: &'a str
}

impl <'a> SubDomains<'a> {
    pub fn new(iter: &str) -> SubDomains {
      SubDomains{ front_offset: iter.len(), iter }
    }
}

impl<'a> Iterator for SubDomains <'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<&'a str> {
    let buf = self.iter.as_bytes();
    if self.front_offset == 0 || self.iter.len() == 0 {
      None
    } else {
      self.front_offset -= 1;
      while buf[self.front_offset] != b'.' && self.front_offset > 0 {
        self.front_offset -= 1;
      }
      if self.front_offset == 0 {
        Some(self.iter)
      } else {
        Some(&self.iter[self.front_offset + 1 ..])
      }
    }
  }
}

#[test]
fn test_normal_domain() {
  let mut subdomains = SubDomains::new("ads.fb.com");
  assert_eq!("com", subdomains.next().unwrap());
  assert_eq!("fb.com", subdomains.next().unwrap());
  assert_eq!("ads.fb.com", subdomains.next().unwrap());
}

pub fn count_char_occurences(line: &str, chr: char) -> usize {
  let mut count: usize = 0;
  for c in line.chars() {
    if chr == c {
      count += 1;
    }
  }
  count
}

pub struct Domain<'a> {
  pub name: &'a str,
  pub dots: usize
}

impl <'a> Domain<'a> {
  pub fn new(name: &str) -> Domain {
    Domain{ name, dots: count_char_occurences(name, '.') }
  }
} 