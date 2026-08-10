use std::collections::HashSet;

use futures::{StreamExt, future::BoxFuture, stream};
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::{Mutex, Semaphore};
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
    semaphore: Semaphore,
}

impl Crawler {
    pub fn new(config: CrawlConfig) -> Result<Self, AppError> {
        let client = build_client(&config.user_agent, config.timeout_secs)?;

        let semaphore = Semaphore::new(config.concurrency.max(1));

        Ok(Crawler {
            config,
            client,
            visited_pages: Mutex::new(HashSet::new()),
            checked_links: Mutex::new(HashSet::new()),
            results: Mutex::new(Vec::new()),
            semaphore,
        })
    }

    pub async fn run(&self) -> CrawlSummary {
        self.crawl_page(self.config.root.clone(), 0).await;

        let results = self.results.lock().await;
        let broken = results.iter().filter(|r| r.status.is_broken()).count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, LinkStatus::Skipped))
            .count();

        CrawlSummary {
            pages_visited: self.visited_pages.lock().await.len(),
            total_checked: results.len(),
            broken,
            skipped,
        }
    }

    pub async fn results(&self) -> Vec<LinkResult> {
        self.results.lock().await.clone()
    }

    fn crawl_page(&self, page_url: Url, depth: usize) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            if !self.mark_visited(&page_url).await {
                return;
            }

            let Some(body) = self.fetch_body(&page_url).await else {
                return;
            };

            let links = extract_links(&body, &page_url);

            let new_links: Vec<Url> = stream::iter(links)
                .filter_map(|link| async move {
                    if self.mark_link_checked(&link).await {
                        Some(link)
                    } else {
                        None
                    }
                })
                .collect()
                .await;

            let checked: Vec<LinkResult> = stream::iter(new_links)
                .map(|link| self.check_one(link, page_url.clone()))
                .buffer_unordered(self.config.concurrency)
                .collect()
                .await;

            let to_recurse: Vec<Url> = if depth < self.config.max_depth {
                checked
                    .iter()
                    .filter(|r| r.kind == LinkKind::Internal && !r.status.is_broken())
                    .map(|r| r.url.clone())
                    .collect()
            } else {
                Vec::new()
            };

            self.results.lock().await.extend(checked);

            stream::iter(to_recurse)
                .for_each_concurrent(self.config.concurrency, |next| {
                    self.crawl_page(next, depth + 1)
                })
                .await;
        })
    }

    async fn fetch_body(&self, page_url: &Url) -> Option<String> {
        let _permit = self.semaphore.acquire().await.ok()?;
        self.client
            .get(page_url.clone())
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()
    }

    async fn check_one(&self, link: Url, page_url: Url) -> LinkResult {
        let kind = classify(&link, &self.config.root);

        if kind == LinkKind::External && !self.config.check_extern {
            return LinkResult {
                url: link.clone(),
                found_on: page_url.clone(),
                kind,
                status: LinkStatus::Skipped,
            };
        }

        let _permit = self
            .semaphore
            .acquire()
            .await
            .expect("semaphore never closed");
        check_link(&self.client, &link, &page_url, kind).await
    }

    async fn mark_visited(&self, url: &Url) -> bool {
        self.visited_pages.lock().await.insert(url.clone())
    }

    async fn mark_link_checked(&self, url: &Url) -> bool {
        self.checked_links.lock().await.insert(url.clone())
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
