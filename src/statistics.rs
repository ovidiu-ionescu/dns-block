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