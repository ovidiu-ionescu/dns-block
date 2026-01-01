use env_logger::Builder;
//use std::collections::HashSet;
use fnv::FnvHashSet as HashSet;

use std::fs::{self, read_to_string};
use std::io::{BufWriter, Write};

use std::sync::mpsc;
use std::thread;

mod cli;
mod dns_resolver;
mod list_of_lists;
mod sub_domains;
use sub_domains::{Domain, count_char_occurences, sub_domain_iterator};
mod filter;
mod statistics;
use statistics::Statistics;

use std::time::Instant;

use rayon::join;

use indoc::indoc;
use log::*;

use mimalloc::MiMalloc;

use crate::cli::{Commands, get_args};
use crate::list_of_lists::fetch_lists;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let command_line_params = get_args();

  setup_logging(command_line_params.debug);

  trace!("{:#?}", command_line_params);

  let start = Instant::now();

  debug!("resolve the remote lists");
  let remote_lists = fetch_lists(command_line_params.lists_file).await?;

  let whitelist_string = match command_line_params.allow_file {
    Some(path) => fs::read_to_string(path).unwrap(),
    _ => String::with_capacity(0),
  };

  debug!("Do the DNS requests for whitelisted domains while we read and sort the domains we want to block");
  let (tx, rx) = mpsc::channel();
  thread::spawn(move || {
    tx.send(expand_whitelist(whitelist_string)).unwrap();
  });

  // domains to blacklist should be processed from shortest
  // to longest

  let start_sorting = start.elapsed().as_millis();

  // read the block files from disk
  // also calculate number of lines
  let mut total_line_count = 0;
  let block_lists = match command_line_params.block_file {
    Some(block_files) => block_files
      .iter()
      .map(|path| {
        let mut text = read_to_string(path).unwrap();
        // converting to lowercase might generate some duplicates
        text.make_ascii_lowercase();
        total_line_count += count_char_occurences(&text, '\n');
        text
      })
      .collect(),
    _ => Vec::new(),
  };

  debug!("count the lines in the list of lists");
  remote_lists.iter().for_each(|fetch_result| {
    if let Ok(text) = &fetch_result.text {
      total_line_count += count_char_occurences(text, '\n');
    }
  });

  debug!("add the local lists to stuff to block");
  let mut bad_domains = Vec::with_capacity(total_line_count);
  block_lists.iter().for_each(|text| {
    for line in text.lines() {
      if let Some(domain) = Domain::new(line) {
        bad_domains.push(domain);
      }
    }
  });

  debug!("add the remote lists to the stuff to block");
  remote_lists.iter().for_each(|fetch_result| {
    if let Ok(text) = &fetch_result.text {
      for line in text.lines() {
        if let Some(domain) = Domain::new(line) {
          bad_domains.push(domain);
        }
      }
    }
  });

  debug!("sort the vector, less dots first");
  let start_sorting_code = start.elapsed().as_millis();
  bad_domains.sort_unstable_by_key(|d: &Domain| d.dots);
  let end_sorting = start.elapsed().as_millis();

  debug!("Prepare the whitelist index");
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
  info!("Statistics total \n{}", Statistics::aggregate(&statistics_com, &statistics_net));

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
  Ok(())
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
  let mut blacklist: HashSet<&str> = HashSet::with_capacity_and_hasher(bad_domains.len() / 2, Default::default());
  let mut whitelisted: HashSet<&str> = HashSet::with_capacity_and_hasher(whitelist.len(), Default::default());
  let mut statistics = Statistics::new();

  for domain in bad_domains.iter().filter(|d| filter_d(d.name)) {
    process_bad_domain(domain.name, &mut blacklist, whitelist, &mut statistics, &mut whitelisted);
  }
  (blacklist, statistics)
}

fn setup_logging(level: u8) {
  let level_filter = match level {
    0 => LevelFilter::Error,
    1 => LevelFilter::Warn,
    2 => LevelFilter::Info,
    3 => LevelFilter::Debug,
    _ => LevelFilter::Trace,
  };
  let mut builder = Builder::from_default_env();
  // if the RUST_LOG var is not defined we use the debug level
  if std::env::var("RUST_LOG").is_err() {
    builder.filter_level(level_filter);
  }
  builder.format_timestamp_secs().init();
}
