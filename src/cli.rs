use std::{fs::metadata, path::PathBuf};

use clap::{Parser, Subcommand, ValueHint};

#[derive(Parser, Debug)]
#[command(name = "dns-block", author, version, about, long_about)]
pub struct Args {
  /// log level, dddd for trace, ddd for debug, dd for info, d for warn, default no output
  #[arg(short, long, action = clap::ArgAction::Count)]
  pub debug: u8,

  /// display timing information at the end of processing
  #[arg(short, long)]
  pub timing: bool,

  #[arg(short, long, num_args = 1.., value_delimiter = ' ', value_hint = ValueHint::FilePath, value_parser = validate_readable_file)]
  pub lists_file: Option<Vec<PathBuf>>,

  /// File containing a list of domains to dns block, multiple can be specified
  #[arg(short, long, num_args = 1.., value_delimiter = ' ', value_hint = ValueHint::FilePath, value_parser = validate_readable_file)]
  pub block_file: Option<Vec<PathBuf>>,

  #[arg(short, long, value_hint = ValueHint::FilePath, value_parser = validate_readable_file)]
  pub allow_file: Option<PathBuf>,

  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
  /// Pack the domains list into one file
  Pack {
    /// output in Bind9 format
    #[arg(short, long)]
    bind: bool,
    /// Output file
    #[arg(name = "output_file", default_value = "simple.blocked")]
    output_file: String,
  },
  /// Act as a pipe when tailing the Bind9 query log
  Pipe {
    /// Filter for just these client IPs (comma separated list)
    #[arg(short, long)]
    filter: Option<String>,
  },
}

pub fn get_args() -> Args {
  Args::parse()
}

fn validate_readable_file(s: &str) -> Result<PathBuf, String> {
  let path = PathBuf::from(s);

  // Check if path exists and is a file
  let meta = metadata(&path).map_err(|_| format!("'{}' does not exist", s))?;

  if !meta.is_file() {
    return Err(format!("'{}' is not a file", s));
  }

  // Check if readable by attempting to open (optional but thorough)
  if std::fs::File::open(&path).is_err() {
    return Err(format!("'{}' is not readable (permission denied)", s));
  }

  Ok(path)
}
