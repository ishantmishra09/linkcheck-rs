#![allow(unused)]

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use url::Url;

use crate::{cli::Cli, error::AppError, types::CrawlConfig};

mod cli;
mod error;
mod types;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = Url::parse(&cli.url).with_context(|| format!("'{}' is not a valid URL", cli.url))?;

    if root.host_str().is_none() {
        return Err(AppError::MissingHost(root.to_string()).into());
    }

    let config = CrawlConfig {
        root: root.clone(),
        max_depth: cli.depth,
        timeout_secs: cli.timeout,
        user_agent: cli.user_agent,
        check_extern: !cli.no_extern,
    };

    println!(
        "{} {} {}",
        "==>".bold().blue(),
        "crawling".bold(),
        root.as_str().cyan()
    );
    println!(
        "depth={} threads={} timeout={}s external={}\n",
        config.max_depth, cli.threads, config.timeout_secs, config.check_extern,
    );

    Ok(())
}
