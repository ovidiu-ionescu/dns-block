//use std::collections::HashSet;
use fnv::FnvHashSet as HashSet;

use std::fs;
use std::io::{BufWriter, Write};

use std::sync::mpsc;
use std::thread;

mod cli;
mod dns_resolver;
mod sub_domains;
use sub_domains::{count_char_occurences, sub_domain_iterator, Domain};
mod filter;
mod statistics;
use statistics::Statistics;

use std::time::Instant;

use rayon::join;

use indoc::indoc;
use log::*;

use mimalloc::MiMalloc;

use crate::cli::Commands;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    let command_line_params = cli::get_cli();

    stderrlog::new()
        .module(module_path!())
        .quiet(false)
        .verbosity(command_line_params.debug as usize)
        .timestamp(stderrlog::Timestamp::Off)
        .init()
        .unwrap();

    trace!("{:#?}", command_line_params);

    let start = Instant::now();

    let domain_block_filename = command_line_params.domain_block_filename;
    let whitelist_filename = command_line_params.domain_whitelist_filename;
    let hosts_blocked_filename = command_line_params.hosts_blocked_filename;

    let whitelist_string = match whitelist_filename.as_ref() {
        "-" => String::with_capacity(0),
        _ => fs::read_to_string(whitelist_filename).unwrap(),
    };

    debug!("Do the DNS requests for whitelisted domains while we read and sort the domains we want to block");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        tx.send(expand_whitelist(whitelist_string)).unwrap();
    });

    let mut domain_block_string = fs::read_to_string(domain_block_filename).unwrap();

    let hosts_blocked_string = match hosts_blocked_filename.as_ref() {
        "-" => String::with_capacity(0),
        _ => fs::read_to_string(hosts_blocked_filename).unwrap(),
    };

    // converting to lowercase might generate some duplicates
    domain_block_string.make_ascii_lowercase();

    // domains to blacklist should be processed from shortest
    // to longest

    let start_sorting = start.elapsed().as_millis();
    // println!("calculate max number of lines");
    let total = count_char_occurences(&domain_block_string, '\n')
        + count_char_occurences(&hosts_blocked_string, '\n');

    // println!("allocate a vector to fit all {} lines", total);
    let mut bad_domains: Vec<Domain> = Vec::with_capacity(total);

    // println!("put all lines from the personal block list in the vector");
    for line in hosts_blocked_string.lines() {
        if let Some(domain) = Domain::new(line) {
            bad_domains.push(domain);
        }
    }

    // println!("put all lines from the public block list in the vector");
    for line in domain_block_string.lines() {
        if let Some(domain) = Domain::new(line) {
            bad_domains.push(domain);
        }
    }

    // println!("sort the vector, less dots first");
    let start_sorting_code = start.elapsed().as_millis();
    bad_domains.sort_unstable_by_key(|d: &Domain| d.dots);
    let end_sorting = start.elapsed().as_millis();

    // Prepare the whitelist index
    // get the cnames from the other thread
    let (whitelist_string, cnames) = rx.recv().unwrap();

    let mut whitelist: HashSet<&str> = HashSet::default();

    for line in whitelist_string.lines() {
        process_whitelist_line(line, &mut whitelist);
    }

    for domain in &cnames {
        process_whitelist_line(domain, &mut whitelist);
    }

    let start_baddies = start.elapsed().as_millis();

    let ((blacklist_com, statistics_com), (blacklist_net, statistics_net)) = join(
        || process_baddies(&bad_domains, &whitelist, |s: &str| s.ends_with("com")),
        || process_baddies(&bad_domains, &whitelist, |s: &str| !s.ends_with("com")),
    );
    info!("Statistics .com \n{}", &statistics_com);
    info!("Statistics .net \n{}", &statistics_net);
    info!(
        "Statistics total \n{}",
        Statistics::aggregate(&statistics_com, &statistics_net)
    );

    match command_line_params.command {
        Commands::Pipe { filter } => {
            filter::filter(&blacklist_com, &blacklist_net, filter.as_deref()).unwrap();
        }
        Commands::Pack { bind, output_file } => {
            let start_writing = start.elapsed().as_millis();
            if bind {
                write_bind_output(&blacklist_com, &blacklist_net, &output_file);
            } else {
                write_output(&blacklist_com, &blacklist_net, &output_file);
            }

            if command_line_params.timing {
                info!(
                    "sorting: {}, sorting core: {}, until after sort: {}, processing baddies: {}",
                    end_sorting - start_sorting,
                    end_sorting - start_sorting_code,
                    start_baddies,
                    start_writing - start_baddies
                );
            }
        }
    }
}

/// Adds a non comment line to the whitelist index
/// It adds the domain and all parent domains
fn process_whitelist_line<'a>(line: &'a str, index: &mut HashSet<&'a str>) {
    if let Some(domain) = Domain::new(line) {
        for seg in sub_domain_iterator(domain.name, 1) {
            index.insert(seg);
        }
        index.insert(domain.name);
    }
}

/// adds a domain to the blocked index if it's not already blocked already or whitelisted
fn process_bad_domain<'a>(
    domain: &'a str,
    index: &mut HashSet<&'a str>,
    whitelist: &HashSet<&'a str>,
    statistics: &mut Statistics,
    whitelisted: &mut HashSet<&'a str>,
) {
    if domain.is_empty() {
        return;
    }
    for seg in sub_domain_iterator(domain, 1) {
        if index.contains(seg) {
            statistics.increment_parent();
            return;
        }
    }
    if !whitelist.contains(domain) {
        if index.insert(domain) {
            statistics.increment_blocked();
        } else {
            statistics.increment_duplicate();
        }
    } else {
        if whitelisted.insert(domain) {
            statistics.increment_distinct_whitelisted();
        }
        debug!("Whitelisted {}", domain);
        statistics.increment_whitelisted();
    }
}

fn write_output(index_com: &HashSet<&str>, index_net: &HashSet<&str>, output_file: &str) {
    let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create(output_file).unwrap());
    let eol: [u8; 1] = [10];
    for d in index_com.iter() {
        f.write_all(d.as_bytes()).unwrap();
        f.write_all(&eol).unwrap();
    }
    for d in index_net.iter() {
        f.write_all(d.as_bytes()).unwrap();
        f.write_all(&eol).unwrap();
    }
    f.flush().unwrap();
}

fn write_bind_output(index_com: &HashSet<&str>, index_net: &HashSet<&str>, output_file: &str) {
    let preamble = indoc! {"
        $TTL 60
        @   IN    SOA  localhost. root.localhost.  (
                2   ; serial 
                3H  ; refresh 
                1H  ; retry 
                1W  ; expiry 
                1H) ; minimum 
            IN    NS    localhost.
    "};
    let prefix = "*.";
    let suffix = " CNAME .";
    let mut f = BufWriter::with_capacity(8 * 1024, fs::File::create(output_file).unwrap());

    f.write_all(preamble.as_bytes()).unwrap();

    let eol: [u8; 1] = [10];
    let mut serialize_index = |index: &HashSet<&str>| {
        for d in index.iter() {
            f.write_all(d.as_bytes()).unwrap();
            f.write_all(suffix.as_bytes()).unwrap();
            f.write_all(&eol).unwrap();

            f.write_all(prefix.as_bytes()).unwrap();
            f.write_all(d.as_bytes()).unwrap();
            f.write_all(suffix.as_bytes()).unwrap();
            f.write_all(&eol).unwrap();
        }
    };
    serialize_index(index_com);
    serialize_index(index_net);

    f.flush().unwrap();
}

// expand the whitelisted domains with their cnames
fn expand_whitelist(whitelist_string: String) -> (String, Vec<String>) {
    // println!("fetch the other domains to whitelist");

    let mut explicit_whitelisted_domains = Vec::with_capacity(50);
    for line in whitelist_string.lines() {
        if let Some(domain) = Domain::new(line) {
            explicit_whitelisted_domains.push(domain.name);
        }
    }
    let mut cnames = Vec::with_capacity(50);
    dns_resolver::resolve_domain(&explicit_whitelisted_domains, &mut cnames).unwrap();
    debug!("Cnames to be whitelisted: {:#?}", cnames);
    (whitelist_string, cnames)
}

/// Makes an index from a list of domains to block
/// filter selects a subset of domains to process, e.g. .com ones
fn process_baddies<'a>(
    bad_domains: &'a [Domain],
    whitelist: &HashSet<&'a str>,
    filter_d: fn(&str) -> bool,
) -> (HashSet<&'a str>, Statistics) {
    let mut blacklist: HashSet<&str> =
        HashSet::with_capacity_and_hasher(bad_domains.len() / 2, Default::default());
    let mut whitelisted: HashSet<&str> =
        HashSet::with_capacity_and_hasher(whitelist.len(), Default::default());
    let mut statistics = Statistics::new();

    for domain in bad_domains.iter().filter(|d| filter_d(d.name)) {
        process_bad_domain(
            domain.name,
            &mut blacklist,
            whitelist,
            &mut statistics,
            &mut whitelisted,
        );
    }
    (blacklist, statistics)
}
