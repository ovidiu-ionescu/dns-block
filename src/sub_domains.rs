pub fn count_char_occurences(line: &str, chr: char) -> usize {
  line.chars().filter(|c| *c == chr).count()
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
