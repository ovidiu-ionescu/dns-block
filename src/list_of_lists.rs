use std::collections::HashSet;
use std::fs::read_to_string;
use std::path::PathBuf;

use futures::future::join_all;
use log::{info, warn};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_retry::RetryTransientMiddleware;
use reqwest_retry::policies::ExponentialBackoff;

type BoxError = Box<dyn std::error::Error>;

pub struct FetchResult {
  pub url: String,
  pub text: reqwest::Result<String>,
}

pub async fn fetch_lists(lists_file: Option<Vec<PathBuf>>) -> Result<Vec<FetchResult>, BoxError> {
  let lists_files = match lists_file {
    Some(l) => {
      if l.is_empty() {
        return Ok(Vec::new());
      } else {
        l
      }
    }
    None => return Ok(Vec::new()),
  };

  let urls: HashSet<String> =
    lists_files.iter().map(|filename| get_url_list(filename)).try_fold(HashSet::new(), |mut acc, set| {
      let set = set?;
      if acc.is_empty() {
        acc = set;
      } else {
        for s in set {
          if !acc.insert(s.clone()) {
            warn!("「{}」is a duplicate", s);
          }
        }
      }
      Ok::<HashSet<String>, BoxError>(acc)
    })?;

  let retry_policy = ExponentialBackoff::builder().build_with_max_retries(10);
  let client = ClientBuilder::new(Client::new()).with(RetryTransientMiddleware::new_with_policy(retry_policy)).build();

  // 1. Create a list of futures (one for each request)
  // We clone the client (it's backed by an Arc, so it's cheap)
  let tasks = urls.into_iter().map(|url| {
    let client = client.clone();
    async move {
      let res = client.get(&url).send().await?;
      let text = res.text().await;
      Ok(FetchResult { url, text })
    }
  });

  // 2. Execute all tasks in parallel
  info!("Fetching URLs...");
  let results: Vec<Result<FetchResult, BoxError>> = join_all(tasks).await;

  let mut ret = Vec::with_capacity(results.len());
  // 4. Filter for successful results and concatenate
  for result in results {
    match result {
      Ok(fetch_result) => match fetch_result.text {
        Ok(ref content) => {
          if has_valid_content(content) {
            ret.push(fetch_result);
          } else {
            warn!("Url 「{}」 returned no valid content", fetch_result.url);
          }
        }
        Err(e) => warn!("Error fetching a url 「{}」: {}", fetch_result.url, e),
      },
      Err(e) => warn!("Error fetching a URL: {}", e),
    }
  }

  Ok(ret)
}

fn get_url_list(filename: &PathBuf) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
  let content = read_to_string(filename)?;
  let mut urls = HashSet::<String>::new();
  content.lines().for_each(|line| {
    let cleaned = match line.split('#').next() {
      Some(it) => it,
      None => return,
    }
    .trim();
    if !cleaned.is_empty() && !urls.insert(cleaned.to_string()) {
      warn!("「{}」 is a duplicate", cleaned);
    }
  });
  if urls.is_empty() { Err("No urls found in file".into()) } else { Ok(urls) }
}

/// check if the list content contains any valid lines
/// this would indicate an invalid list
/// TODO replace to use Domain
fn has_valid_content(content: &str) -> bool {
  content
    .lines()
    .filter_map(|line| {
      let cleaned = line.split('#').next()?.trim();
      if cleaned.is_empty() { None } else { Some(cleaned.to_string()) }
    })
    .any(|line| {
      match line.split(' ').next_back() {
        Some(host) => match addr::parse_dns_name(host) {
          Ok(_) => Some(host),
          Err(_) => None,
        },
        _ => None,
      }
      .is_some()
    })
}
