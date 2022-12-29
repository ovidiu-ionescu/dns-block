use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "dns-block", author, version, about, long_about)]
pub struct Cli {
    /// log level, dddd for trace, ddd for debug, dd for info, d for warn, default no output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub debug: u8,

    /// display timing information at the end of processing
    #[arg(short, long)]
    pub timing: bool,

    /// File containing the list of domains to dns block.
    #[arg(name = "domains.blocked", value_parser = file_exists)]
    pub domain_block_filename: String,

    /// File containing the list of domains to whitelist. Use - to skip this parameter
    #[arg(name = "domains.whitelist", value_parser = file_exists)]
    pub domain_whitelist_filename: String,

    /// Additional personal file with domains to block. Use - to skip this parameter
    #[arg(name = "hosts_blocked.txt", value_parser = file_exists)]
    pub hosts_blocked_filename: String,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Pack the domains list into one file
    Pack {
        /// output in Bind9 format
        #[arg(short, long)]
        bind:        bool,
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

pub fn get_cli() -> Cli {
    Cli::parse()
}

fn file_exists(path: &str) -> Result<String, String> {
    if "-" == path || std::fs::metadata(path).is_ok() {
        Ok(path.to_string())
    } else {
        Err(format!("{path}: No such file or directory"))
    }
}
