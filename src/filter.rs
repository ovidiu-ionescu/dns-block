use crate::sub_domains::sub_domain_iterator;
use fnv::FnvHashSet as HashSet;
use log::*;
use std::io::{self, Write};

fn is_domain_blocked_by_index(domain: &str, index: &HashSet<&str>) -> bool {
    for seg in sub_domain_iterator(domain, 1) {
        if index.contains(seg) {
            return true;
        }
    }
    false
}

fn is_domain_blocked(
    domain: &str,
    blacklist_com: &HashSet<&str>,
    blacklist_net: &HashSet<&str>,
) -> bool {
    if domain.ends_with("com") {
        is_domain_blocked_by_index(domain, blacklist_com)
    } else {
        is_domain_blocked_by_index(domain, blacklist_net)
    }
}

fn extract<'a>(line: &'a str, pref: &str, suf: &str) -> Option<&'a str> {
    if let Some(index_pref) = line.find(pref) {
        let start = index_pref + pref.len();
        if let Some(end) = line[start..].find(suf) {
            let data = &line[start..start + end];
            return Some(data);
        }
    }
    None
}

pub fn filter(
    blacklist_com: &HashSet<&str>,
    blacklist_net: &HashSet<&str>,
    filter_parameter: Option<&str>,
) -> io::Result<()> {
    debug!("Filter for client ips: {:#?}", filter_parameter);
    let mut input = String::new();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let ip_filter: HashSet<&str> = match filter_parameter {
        Some(filter) => filter.split(',').collect::<HashSet<&str>>(),
        None => HashSet::with_capacity_and_hasher(0, Default::default()),
    };

    loop {
        let n = io::stdin().read_line(&mut input)?;
        if n == 0 {
            return Ok(());
        }
        let domain_opt = extract(&input, "query: ", " ");
        let client_opt = extract(&input, "client ", "#");
        if let (Some(domain), Some(client)) = (domain_opt, client_opt) {
            if ip_filter.is_empty() || ip_filter.contains(&client) {
                if !is_domain_blocked(domain, blacklist_com, &blacklist_net) {
                    handle.write_all(input.as_bytes())?;
                } else {
                    handle.write_fmt(format_args!("{} {} {}\n", &client, &domain, "blocked"))?;
                }
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
        assert_eq!(
            "mydomain.com",
            super::extract(&line, "query: ", " ").unwrap()
        );
    }
}
