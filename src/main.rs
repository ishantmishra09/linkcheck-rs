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

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cli.worker_threads.max(1))
        .enable_all()
        .build()
        .map_err(|e| AppError::Runtime(e.to_string()))
        .context("failed to start tokio runtime")?;

    runtime.block_on(run(cli, root))
}

async fn run(cli: Cli, root: Url) -> Result<()> {
    let config = CrawlConfig {
        root: root.clone(),
        max_depth: cli.depth,
        timeout_secs: cli.timeout,
        user_agent: cli.user_agent,
        check_extern: !cli.no_extern,
        concurrency: cli.concurrency,
    };

    println!(
        "{} {} {}",
        "==>".bold().blue(),
        "crawling".bold(),
        root.as_str().cyan()
    );
    println!(
        "depth={} concurrency={} timeout={}s external={}\n",
        config.max_depth, cli.concurrency, config.timeout_secs, config.check_extern,
    );

    let crawler = Crawler::new(config).context("failed to set up crawler")?;
    let summary = crawler.run().await;

    let mut results = crawler.results().await;
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
