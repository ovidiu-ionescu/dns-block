pub fn count_char_occurences(line: &str, chr: char) -> usize {
  let mut count: usize = 0;
  for c in line.chars() {
    if chr == c {
      count += 1;
    }
  }
  count
}
pub fn count_char_occurences_and_lowercase(line: &mut str, chr: char) -> usize {
  let mut count: usize = 0;
  for mut c in &mut line.chars() {
    if chr == c {
      count += 1;
    } 
    // else {
    //   if c.is_ascii_uppercase() {
    //     c.make_ascii_lowercase();
    //   }
    // }
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

fn iterate_subs(domain: &str, min: usize) {
  // for (i, c) in domain.chars().rev().enumerate().filter(|(i, c)| *i == 0 || *c == '.').skip(min) {
  // for (i, c) in domain.char_indices().rev().filter(|(i, c)| *i == 0 || *c == '.').skip(min)  {
  //   println!("iter {} {}", i, &domain[i..]);
  // }
  for seg in domain.char_indices().rev().filter(|(i, c)| *i == 0 || *c == '.').skip(min)
    .map(|(i, _c)| if 0 == i { &domain[i..] } else { &domain[i + 1 ..]})  {
    println!("seg {}", seg);
  }
}

#[test]
fn test_iter() {
  iterate_subs("many.ads.fb.com", 1);
  iterate_subs("aha.org", 1);
}
