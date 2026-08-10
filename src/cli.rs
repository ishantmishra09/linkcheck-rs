use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "linkcheck-rs", version, about, long_about = None)]
pub struct Cli {
    pub url: String,

    #[arg(short, long, default_value_t = 2)]
    pub depth: usize,

    #[arg(short = 'c', long, default_value_t = 8)]
    pub concurrency: usize,

    #[arg(short = 'w', long, default_value_t = 4)]
    pub worker_threads: usize,

    #[arg(long, default_value_t = 10)]
    pub timeout: u64,

    #[arg(long)]
    pub no_extern: bool,

    #[arg(long, default_value_t = format!("linkcheck-rs/{}", env!("CARGO_PKG_VERSION")))]
    pub user_agent: String,

    #[arg(short, long)]
    pub quiet: bool,
}
