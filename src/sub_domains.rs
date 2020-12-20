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

pub fn sub_domain_iterator<'a>(domain: &'a str, min: usize) -> impl Iterator<Item = &'a str> {
  domain.char_indices().rev()
    .filter(|(_i, c)| *c == '.')
    .skip(min)
    .map(move |(i, _c)| &domain[i + 1 ..])
}

#[test]
fn sub_domain_iterator_test() {
  let mut subdomains = sub_domain_iterator("many.ads.fb.com", 1);
  assert_eq!("fb.com", subdomains.next().unwrap());
  assert_eq!("ads.fb.com", subdomains.next().unwrap());
  assert_eq!(None, subdomains.next());
}

