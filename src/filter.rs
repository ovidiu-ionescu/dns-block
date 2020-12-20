
use std::io::{ self, Write };
use fnv::FnvHashSet as HashSet;
use crate::sub_domains::{ sub_domain_iterator };

fn is_domain_blocked_by_index(domain: &str, index: &HashSet<&str>) -> bool {
  for seg in sub_domain_iterator(domain, 1) { 
    if index.contains(seg) {
      return true;
    }
  }
  false
}

fn is_domain_blocked(domain: &str, blacklist_com: &HashSet<&str>, blacklist_net: &HashSet<&str>) -> bool {
    if domain.ends_with("com") {
        is_domain_blocked_by_index(domain, blacklist_com)
    } else {
        is_domain_blocked_by_index(domain, blacklist_net)
    }
}

pub fn filter(blacklist_com: &HashSet<&str>, blacklist_net: &HashSet<&str>) -> io::Result<()> {
    let crit = "query: ";
    let mut input = String::new();

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    loop {
        let n = io::stdin().read_line(&mut input)?;
        if n == 0 {
            return Ok(())
        }
        
        if let Some(p) = input.find(crit) {
            let start = p + crit.len();
            if let Some(end) = input[start ..].find(" ") {
                let domain = &input[start ..start + end];
                if !is_domain_blocked(domain, blacklist_com, &blacklist_net) {
                    handle.write_all(input.as_bytes())?;
                }
            }
        }
        input.truncate(0);
    }
}