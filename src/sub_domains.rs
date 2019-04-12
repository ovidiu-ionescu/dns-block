
pub struct SubDomains<'a> {
  front_offset: usize,
  skip: u8,
  domain: &'a str
}

impl <'a> SubDomains<'a> {
    pub fn new(domain: &str, skip: u8) -> SubDomains {
      SubDomains{ front_offset: domain.len(), skip, domain }
    }
}

impl<'a> Iterator for SubDomains <'a> {
  type Item = &'a str;

  fn next(&mut self) -> Option<&'a str> {
    let buf = self.domain.as_bytes();
    
    // empty string or we're already finished iterating
    if self.domain.len() == 0 || self.front_offset == 0 {
      return None;
    } 
    
    self.front_offset -= 1;
    // if we are at the beginning skip dots
    while self.skip > 0 && self.front_offset > 0 {
      if buf[self.front_offset] == b'.' {
        self.skip -= 1;
      }
      self.front_offset -= 1;
    }

    // if the string does not have enough dots return
    if self.skip > 0 {
      return None;
    }

    // find the next dot
    while self.front_offset > 0 && buf[self.front_offset] != b'.' {
      self.front_offset -= 1;
    }

    if self.front_offset == 0 {
      Some(self.domain)
    } else {
      Some(&self.domain[self.front_offset + 1 ..])
    }
  }
}

#[test]
fn test_normal_domain() {
  let mut subdomains = SubDomains::new("ads.fb.com", 1);
  // assert_eq!("com", subdomains.next().unwrap());
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
