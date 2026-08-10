use reqwest::{Client, Response};
use url::Url;

use crate::{
    error::AppError,
    types::{LinkKind, LinkResult, LinkStatus},
};

pub fn build_client(user_agent: &str, timeout_secs: u64) -> Result<Client, AppError> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(std::time::Duration::from_secs(timeout_secs))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| AppError::ClientBuild(e.to_string()))
}

pub fn classify(link: &Url, root: &Url) -> LinkKind {
    match (link.host_str(), root.host_str()) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => LinkKind::Internal,
        _ => LinkKind::External,
    }
}

pub async fn check_link(client: &Client, link: &Url, found_on: &Url, kind: LinkKind) -> LinkResult {
    let status = match probe(client, link).await {
        Ok(code) if (200..400).contains(&code) => LinkStatus::Ok(code),
        Ok(code) => LinkStatus::Broken(code),
        Err(e) => LinkStatus::Failed(describe(&e)),
    };

    LinkResult {
        url: link.clone(),
        found_on: found_on.clone(),
        kind,
        status,
    }
}

async fn probe(client: &Client, link: &Url) -> Result<u16, AppError> {
    match client.head(link.clone()).send().await {
        Ok(resp) => {
            let code = resp.status().as_u16();
            if code == 405 || code == 501 {
                get_status(client, link).await
            } else {
                Ok(code)
            }
        }
        Err(_) => get_status(client, link).await,
    }
}

async fn get_status(client: &Client, link: &Url) -> Result<u16, AppError> {
    let resp: Response = client.get(link.clone()).send().await?;
    Ok(resp.status().as_u16())
}

fn describe(err: &AppError) -> String {
    match err {
        AppError::Request(e) if e.is_timeout() => "timed out".to_string(),
        AppError::Request(e) if e.is_connect() => "connection failed".to_string(),
        AppError::Request(e) => e.to_string(),
        other => other.to_string(),
    }
}
