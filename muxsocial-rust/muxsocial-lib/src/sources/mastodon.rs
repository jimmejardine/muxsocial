//! Mastodon source client — a thin, read-only REST client.
//!
//! There is no WASM-capable Mastodon Rust library (the existing crates are all
//! tokio+reqwest, native-only), but the public REST API is plain JSON over
//! HTTPS, so we roll our own over [`crate::http::HttpTransport`]. Reading public
//! statuses needs no authentication: resolve the account, then list its
//! statuses.
//!
//! CORS caveat: most instances do not send permissive CORS headers, so these
//! calls will be blocked from a real browser and will need a proxy. Native
//! (integration test / harness) calls are unaffected.
//!
//! Paging is by status id: [`MastodonPager`] uses the API's `min_id` (newer)
//! and `max_id` (older) query params, where the cursor is a status id (our
//! `source_post_id`).

use anyhow::Context;
use serde::Deserialize;

use crate::http::{DefaultHttpTransport, HttpRequest, HttpTransport};
use crate::post::{AggregatedPost, SourceNetwork, parse_rfc3339_to_epoch_millis};
use crate::timeline::SourcePager;

/// Fetch up to `limit` recent public statuses for a fediverse `account` in the
/// usual concatenated form `@user@instance` (e.g. `"@Gargron@mastodon.social"`;
/// a leading `@` is optional). Unauthenticated.
pub async fn fetch_recent_posts(http_transport: &impl HttpTransport, account: &str, limit: u32) -> anyhow::Result<Vec<AggregatedPost>> {
    let (instance_base_url, account_id) = resolve_account(http_transport, account).await?;
    fetch_statuses(http_transport, &instance_base_url, &account_id, limit, None, None).await
}

/// Resolve a `@user@instance` handle to its instance base URL and numeric
/// account id (the id is what the statuses endpoint is keyed by).
async fn resolve_account(http_transport: &impl HttpTransport, account: &str) -> anyhow::Result<(String, String)> {
    let (instance_base_url, username) = split_fediverse_handle(account)?;

    let lookup_url = format!("{instance_base_url}/api/v1/accounts/lookup?acct={username}");
    let lookup_response = http_transport.execute(HttpRequest::get(lookup_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(lookup_response.is_success(), "Mastodon account lookup for {account:?} failed: HTTP {}", lookup_response.status);
    let resolved_account: MastodonAccount = serde_json::from_slice(&lookup_response.body).context("parsing Mastodon account lookup response")?;

    Ok((instance_base_url, resolved_account.id))
}

/// List `account_id`'s recent statuses (no boosts), optionally bounded by
/// `max_id` (older than) and/or `min_id` (newer than).
async fn fetch_statuses(http_transport: &impl HttpTransport, instance_base_url: &str, account_id: &str, limit: u32, max_id: Option<&str>, min_id: Option<&str>) -> anyhow::Result<Vec<AggregatedPost>> {
    log::info!("mastodon: fetching up to {limit} statuses for account {account_id} at {instance_base_url} (max_id={max_id:?}, min_id={min_id:?})");

    let mut statuses_url = format!("{instance_base_url}/api/v1/accounts/{account_id}/statuses?limit={limit}&exclude_reblogs=true");
    if let Some(max_id) = max_id {
        statuses_url.push_str(&format!("&max_id={max_id}"));
    }
    if let Some(min_id) = min_id {
        statuses_url.push_str(&format!("&min_id={min_id}"));
    }

    let statuses_response = http_transport.execute(HttpRequest::get(statuses_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(statuses_response.is_success(), "Mastodon statuses fetch failed: HTTP {}", statuses_response.status);
    let statuses: Vec<MastodonStatus> = serde_json::from_slice(&statuses_response.body).context("parsing Mastodon statuses response")?;
    log::debug!("mastodon: received {} status(es) for account {account_id}", statuses.len());

    statuses.into_iter().map(map_status).collect()
}

/// A timeline pager for a single Mastodon account. Paginates by status id.
pub struct MastodonPager {
    http_transport: DefaultHttpTransport,
    account: String,
    /// Cached `(instance_base_url, account_id)` after the first resolve.
    resolved: Option<(String, String)>,
}

impl MastodonPager {
    pub fn new(http_transport: DefaultHttpTransport, account: impl Into<String>) -> Self {
        Self {
            http_transport,
            account: account.into(),
            resolved: None,
        }
    }

    async fn ensure_resolved(&mut self) -> anyhow::Result<(String, String)> {
        if self.resolved.is_none() {
            self.resolved = Some(resolve_account(&self.http_transport, &self.account).await?);
        }
        Ok(self.resolved.clone().expect("just resolved"))
    }
}

impl SourcePager for MastodonPager {
    async fn fetch_newer(&mut self, newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let (instance_base_url, account_id) = self.ensure_resolved().await?;
        let min_id = newest_known.map(|post| post.source_post_id.as_str());
        fetch_statuses(&self.http_transport, &instance_base_url, &account_id, limit as u32, None, min_id).await
    }

    async fn fetch_older(&mut self, oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let (instance_base_url, account_id) = self.ensure_resolved().await?;
        let max_id = oldest_known.map(|post| post.source_post_id.as_str());
        fetch_statuses(&self.http_transport, &instance_base_url, &account_id, limit as u32, max_id, None).await
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        // The resolved account id is stable; nothing to clear.
        Ok(())
    }
}

/// Split a concatenated fediverse handle (`@user@instance` or `user@instance`)
/// into the instance base URL (`https://instance`) and the local username.
fn split_fediverse_handle(account: &str) -> anyhow::Result<(String, String)> {
    let handle = account.trim().trim_start_matches('@');
    let (username, instance_host) = handle.split_once('@').with_context(|| format!("expected a \"@user@instance\" Mastodon handle, got {account:?}"))?;
    anyhow::ensure!(!username.is_empty() && !instance_host.is_empty(), "invalid Mastodon handle {account:?}");
    Ok((format!("https://{}", instance_host.trim_end_matches('/')), username.to_string()))
}

#[derive(Deserialize)]
struct MastodonAccount {
    id: String,
}

#[derive(Deserialize)]
struct MastodonStatus {
    id: String,
    created_at: String,
    content: String,
    account: MastodonStatusAccount,
}

#[derive(Deserialize)]
struct MastodonStatusAccount {
    acct: String,
    display_name: String,
}

fn map_status(status: MastodonStatus) -> anyhow::Result<AggregatedPost> {
    let created_at_millis = parse_rfc3339_to_epoch_millis(&status.created_at)?;
    let author_display_name = if status.account.display_name.is_empty() { None } else { Some(status.account.display_name) };

    Ok(AggregatedPost {
        source: SourceNetwork::Mastodon,
        source_post_id: status.id,
        author_identifier: status.account.acct,
        author_display_name,
        created_at_millis,
        // Mastodon status content is sanitized HTML.
        content_html: status.content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_handle_with_leading_at() {
        let (instance_base_url, username) = split_fediverse_handle("@Gargron@mastodon.social").expect("should split");
        assert_eq!(instance_base_url, "https://mastodon.social");
        assert_eq!(username, "Gargron");
    }

    #[test]
    fn splits_handle_without_leading_at() {
        let (instance_base_url, username) = split_fediverse_handle("Gargron@mastodon.social").expect("should split");
        assert_eq!(instance_base_url, "https://mastodon.social");
        assert_eq!(username, "Gargron");
    }

    #[test]
    fn rejects_handle_without_instance() {
        assert!(split_fediverse_handle("@Gargron").is_err());
        assert!(split_fediverse_handle("Gargron").is_err());
    }
}
