pub fn count_char_occurences(line: &str, chr: char) -> usize {
  line.chars().filter(|c| *c == chr).count()
}

#[derive(Debug)]
pub struct Domain<'a> {
  pub name: &'a str,
  pub dots: usize
}

impl <'a> Domain<'a> {
  pub fn new(line: &str) -> Option<Domain> {
    let comment_stripped = match line.find('#') {
      Some(idx) => &line[0 .. idx],
      None => line
    }.trim();
    if let Some(name) = comment_stripped.split_whitespace().rev().next() {
      let dots = count_char_occurences(name, '.');
      if dots > 0 {
        return Some(Domain{ name, dots })
      }
    }
    None
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

#[test]
fn domain_constructor_test() {

  let nd = vec![
    "# just a comment",
    "   # just a comment",
    "localhost",
  ];

  for line in &nd {
    let od = Domain::new(line);
    assert!(od.is_none());
  }

  let v = vec![
    "domain.com",
    "domain.com # domain and comment",
    "10.0.0.1 domain.com",
    " 10.0.0.1  domain.com # comment"
  ];

  for line in &v {
    let od = Domain::new(line);
    assert!(od.is_some());
    let d = od.unwrap();
    assert_eq!("domain.com", d.name);
    assert_eq!(1, d.dots);
  }
}

