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

use anyhow::Context;
use serde::Deserialize;

use crate::http::{HttpRequest, HttpTransport};
use crate::post::{AggregatedPost, SourceNetwork, parse_rfc3339_to_epoch_millis};

/// Fetch up to `limit` recent public statuses for a fediverse `account` in the
/// usual concatenated form `@user@instance` (e.g. `"@Gargron@mastodon.social"`;
/// a leading `@` is optional). Unauthenticated.
pub async fn fetch_recent_posts(http_transport: &impl HttpTransport, account: &str, limit: u32) -> anyhow::Result<Vec<AggregatedPost>> {
    // The user-facing identifier is a single `@user@instance` handle; the split
    // into an instance base URL and a local username is purely internal.
    let (instance_base_url, username) = split_fediverse_handle(account)?;
    log::info!("mastodon: fetching up to {limit} statuses for {username}@{instance_base_url}");

    // 1. Resolve the account id from its local username.
    let lookup_url = format!("{instance_base_url}/api/v1/accounts/lookup?acct={username}");
    let lookup_response = http_transport.execute(HttpRequest::get(lookup_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(lookup_response.is_success(), "Mastodon account lookup for {account:?} failed: HTTP {}", lookup_response.status);
    let resolved_account: MastodonAccount = serde_json::from_slice(&lookup_response.body).context("parsing Mastodon account lookup response")?;

    // 2. List that account's recent statuses (no boosts, to keep authorship clean).
    let statuses_url = format!("{instance_base_url}/api/v1/accounts/{}/statuses?limit={limit}&exclude_reblogs=true", resolved_account.id);
    let statuses_response = http_transport.execute(HttpRequest::get(statuses_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(statuses_response.is_success(), "Mastodon statuses fetch failed: HTTP {}", statuses_response.status);
    let statuses: Vec<MastodonStatus> = serde_json::from_slice(&statuses_response.body).context("parsing Mastodon statuses response")?;
    log::debug!("mastodon: received {} status(es) for {username}@{instance_base_url}", statuses.len());

    statuses.into_iter().map(map_status).collect()
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
        content_text: status.content,
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
