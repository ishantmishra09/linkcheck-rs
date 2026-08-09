mod cli;
mod crawler;
mod error;
mod request;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use url::Url;

use crate::{
    cli::Cli,
    crawler::Crawler,
    error::AppError,
    types::{CrawlConfig, LinkStatus},
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let root = Url::parse(&cli.url).with_context(|| format!("'{}' is not a valid URL", cli.url))?;

    if root.host_str().is_none() {
        return Err(AppError::MissingHost(root.to_string()).into());
    }

    rayon::ThreadPoolBuilder::new()
        .num_threads(cli.threads)
        .build_global()
        .map_err(|e| AppError::Threadpool(e.to_string()))
        .context("failed to initialize rayon thread pool")?;

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

    let crawler = Crawler::new(config).context("failed to set up crawler")?;
    let summary = crawler.run();

    let mut results = crawler.results();
    results.sort_by(|a, b| a.found_on.as_str().cmp(b.found_on.as_str()));

    for result in &results {
        match &result.status {
            LinkStatus::Ok(_) if !cli.quiet => {
                println!("{}  {}", " OK ".on_green().white(), result)
            }
            LinkStatus::Skipped if !cli.quiet => println!("{}  {}", "SKIP".dimmed(), result),
            LinkStatus::Broken(_) | LinkStatus::Failed(_) => {
                println!("{}  {}", "FAIL".on_red().white(), result)
            }
            _ => {}
        }
    }

    println!("\n{} {}", "==>".bold().blue(), summary);

    if summary.broken > 0 {
        println!(
            "{}",
            format!("{} broken link(s) found", summary.broken)
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "no broken links found".green().bold());

    Ok(())
}
