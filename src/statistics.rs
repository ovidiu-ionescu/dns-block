use std::fmt;

#[derive(Debug)]
pub struct Statistics {
    parent: usize,
    duplicate: usize,
    whitelisted: usize,
    blocked: usize,
}

impl Statistics {
  pub fn new() -> Statistics {
    Statistics {
        parent: 0,
        duplicate: 0,
        whitelisted: 0,
        blocked: 0,
    }
  }

  pub fn increment_parent(&mut self) {
      self.parent += 1;
  }

  pub fn increment_duplicate(&mut self) {
      self.duplicate += 1;
  }

  pub fn increment_whitelisted(&mut self) {
      self.whitelisted += 1;
  }

  pub fn increment_blocked(&mut self) {
      self.blocked += 1;
  }

  pub fn aggregate(stat1: &Statistics, stat2: &Statistics) -> Statistics {
    Statistics {
      parent: stat1.parent + stat2.parent,
      duplicate: stat1.duplicate + stat2.duplicate,
      whitelisted: stat1.whitelisted + stat2.whitelisted,
      blocked: stat1.blocked + stat2.blocked,
    }
  }
}

impl fmt::Display for Statistics {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let total = self.parent + self.duplicate + self.whitelisted + self.blocked;
    let pct = |x: usize| x as f32 * 100.0 / total as f32;
    write!(f, indoc::indoc! {"
      Subdomains:  {:>7} {:>6.2}%
      Duplicates:  {:>7} {:>6.2}%
      Whitelisted: {:>7} {:>6.2}%
      Blocked:     {:>7} {:>6.2}%
      Total:       {:>7} 100.00%
    "}, 
    self.parent, pct(self.parent), 
    self.duplicate, pct(self.duplicate),
    self.whitelisted, pct(self.whitelisted),
    self.blocked, pct(self.blocked),
    total)
  }
}

#[cfg(test)]
mod tests_display {

  #[test]
  fn format_test() {
    let s = super::Statistics {
      parent: 101,
      duplicate: 201,
      whitelisted: 301,
      blocked: 401,
    };

    assert_eq!(indoc::indoc! {"
      Subdomains:      101  10.06%
      Duplicates:      201  20.02%
      Whitelisted:     301  29.98%
      Blocked:         401  39.94%
      Total:          1004 100.00%
    "}, format!("{}", s));
  }

}