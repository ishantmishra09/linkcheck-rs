use std::fmt;

use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Internal,
    External,
}

impl fmt::Display for LinkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal => write!(f, "Internal"),
            Self::External => write!(f, "External"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum LinkStatus {
    Ok(u16),
    Broken(u16),
    Failed(String),
    Skipped,
}

impl LinkStatus {
    pub fn is_broken(&self) -> bool {
        matches!(self, LinkStatus::Broken(_) | LinkStatus::Failed(_))
    }
}

impl fmt::Display for LinkStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkStatus::Ok(code) => write!(f, "OK ({code})"),
            LinkStatus::Broken(code) => write!(f, "BROKEN ({code})"),
            LinkStatus::Failed(msg) => write!(f, "FAILED ({msg})"),
            LinkStatus::Skipped => write!(f, "SKIPPED"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinkResult {
    pub url: Url,
    pub found_on: Url,
    pub kind: LinkKind,
    pub status: LinkStatus,
}

impl fmt::Display for LinkResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {} (found on {}) ",
            self.kind, self.url, self.status, self.found_on
        )
    }
}

#[derive(Debug, Clone)]
pub struct CrawlConfig {
    pub root: Url,
    pub max_depth: usize,
    pub timeout_secs: u64,
    pub user_agent: String,
    pub check_extern: bool,
}

impl Default for CrawlConfig {
    fn default() -> Self {
        CrawlConfig {
            root: Url::parse("http://localhost").expect("static URL is valid"),
            max_depth: 2,
            timeout_secs: 10,
            user_agent: format!("linkcheck-rs/{}", env!("CARGO_PKG_VERSION")),
            check_extern: true,
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CrawlSummary {
    pub pages_visited: usize,
    pub total_checked: usize,
    pub broken: usize,
    pub skipped: usize,
}

impl fmt::Display for CrawlSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pages visited: {} | links checked: {} | broken: {} | skipped: {} ",
            self.pages_visited, self.total_checked, self.broken, self.skipped
        )
    }
}
