
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

fn extract<'a>(line: &'a str, pref: &str, suf: &str) -> Option<&'a str> {
        if let Some(index_pref) = line.find(pref) {
            let start = index_pref + pref.len();
            if let Some(end) = line[start ..].find(suf) {
                let data = &line[start ..start + end];
                return Some(data);
            }
        }
        None
}

pub fn filter(blacklist_com: &HashSet<&str>, blacklist_net: &HashSet<&str>) -> io::Result<()> {
    let mut input = String::new();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    loop {
        let n = io::stdin().read_line(&mut input)?;
        if n == 0 {
            return Ok(())
        }
        let domain_opt = extract(&input, "query: ", " ");
        let client_opt  = extract(&input, "client ", "#");
        if domain_opt.is_some() && client_opt.is_some() {
            let domain = domain_opt.unwrap();
            let client = client_opt.unwrap();
            if !is_domain_blocked(domain, blacklist_com, &blacklist_net) {
                handle.write_all(input.as_bytes())?;
            } else {
                handle.write_fmt(format_args!("{} {} {}\n", &client, &domain, "blocked"))?;
            }
        }
        
        input.truncate(0);
    }
}

#[cfg(test)]
mod tests_filter {
    #[test]
    fn extraction_test() {
        let line = "20-Jan-2021 10:10:10.536 client 10.0.0.30#7216 (mydomain.com): view internal: query: mydomain.com IN A + (10.0.0.12)";
        assert_eq!("10.0.0.30", super::extract(&line, "client ", "#").unwrap());
        assert_eq!("mydomain.com", super::extract(&line, "query: ", " ").unwrap());
    }
}
