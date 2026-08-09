use std::{collections::HashSet, sync::Mutex};

use rayon::prelude::*;
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use url::Url;

use crate::{
    error::AppError,
    request::{build_client, check_link, classify},
    types::{CrawlConfig, CrawlSummary, LinkKind, LinkResult, LinkStatus},
};

pub struct Crawler {
    config: CrawlConfig,
    client: Client,
    visited_pages: Mutex<HashSet<Url>>,
    checked_links: Mutex<HashSet<Url>>,
    results: Mutex<Vec<LinkResult>>,
}

impl Crawler {
    pub fn new(config: CrawlConfig) -> Result<Self, AppError> {
        let client = build_client(&config.user_agent, config.timeout_secs)?;

        Ok(Crawler {
            config,
            client,
            visited_pages: Mutex::new(HashSet::new()),
            checked_links: Mutex::new(HashSet::new()),
            results: Mutex::new(Vec::new()),
        })
    }

    pub fn run(&self) -> CrawlSummary {
        self.crawl_page(self.config.root.clone(), 0);

        let results = self.results.lock().expect("results mutex poisoned");
        let broken = results.iter().filter(|r| r.status.is_broken()).count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, LinkStatus::Skipped))
            .count();

        CrawlSummary {
            pages_visited: self
                .visited_pages
                .lock()
                .expect("visited mutex poisoned")
                .len(),
            total_checked: results.len(),
            broken,
            skipped,
        }
    }

    pub fn results(&self) -> Vec<LinkResult> {
        self.results.lock().expect("results mutex poisoned").clone()
    }

    fn crawl_page(&self, page_url: Url, depth: usize) {
        // Prevent crawling the same page more than once
        if !self.mark_visited(&page_url) {
            return;
        }

        let Ok(body) = self
            .client
            .get(page_url.clone())
            .send()
            .and_then(|r| r.text())
        else {
            return;
        };

        let links = extract_links(&body, &page_url);

        // only check links that have not already been checked globally.
        let new_links: Vec<Url> = links
            .into_iter()
            .filter(|link| self.mark_link_checked(link))
            .collect();

        let checked: Vec<LinkResult> = new_links
            .par_iter()
            .map(|link| self.check_one(link, &page_url))
            .collect();

        let to_recurse: Vec<Url> = if depth < self.config.max_depth {
            checked
                .iter()
                .filter(|r| r.kind == LinkKind::Internal && !r.status.is_broken())
                .map(|r| r.url.clone())
                .collect()
        } else {
            Vec::new()
        };

        self.results
            .lock()
            .expect("results mutex poisoned")
            .extend(checked);

        to_recurse
            .into_par_iter()
            .for_each(|next| self.crawl_page(next, depth + 1));
    }

    fn check_one(&self, link: &Url, page_url: &Url) -> LinkResult {
        let kind = classify(link, &self.config.root);

        if kind == LinkKind::External && !self.config.check_extern {
            return LinkResult {
                url: link.clone(),
                found_on: page_url.clone(),
                kind,
                status: LinkStatus::Skipped,
            };
        }

        check_link(&self.client, link, page_url, kind)
    }

    fn mark_visited(&self, url: &Url) -> bool {
        self.visited_pages
            .lock()
            .expect("visited mutex poisoned")
            .insert(url.clone())
    }

    fn mark_link_checked(&self, url: &Url) -> bool {
        self.checked_links
            .lock()
            .expect("checked links mutex poisoned")
            .insert(url.clone())
    }
}

fn extract_links(html: &str, base: &Url) -> Vec<Url> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("a[href]").expect("static selector is valid");

    let mut seen = HashSet::new();
    document
        .select(&selector)
        .filter_map(|el| el.value().attr("href"))
        .filter_map(|href| base.join(href).ok())
        .filter(|url| matches!(url.scheme(), "http" | "https"))
        .filter_map(|mut url| {
            url.set_fragment(None);
            seen.insert(url.clone()).then_some(url)
        })
        .collect()
}
