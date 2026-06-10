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

/// Fetch up to `limit` recent public statuses for `account_acct` (e.g.
/// `"Gargron"` or `"user@instance"`) on the Mastodon instance at
/// `instance_base_url` (e.g. `"https://mastodon.social"`). Unauthenticated.
pub async fn fetch_recent_posts(http_transport: &impl HttpTransport, instance_base_url: &str, account_acct: &str, limit: u32) -> anyhow::Result<Vec<AggregatedPost>> {
    let base_url = instance_base_url.trim_end_matches('/');

    // 1. Resolve the account id from its acct handle.
    let lookup_url = format!("{base_url}/api/v1/accounts/lookup?acct={account_acct}");
    let lookup_response = http_transport.execute(HttpRequest::get(lookup_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(lookup_response.is_success(), "Mastodon account lookup for {account_acct:?} failed: HTTP {}", lookup_response.status);
    let account: MastodonAccount = serde_json::from_slice(&lookup_response.body).context("parsing Mastodon account lookup response")?;

    // 2. List that account's recent statuses (no boosts, to keep authorship clean).
    let statuses_url = format!("{base_url}/api/v1/accounts/{}/statuses?limit={limit}&exclude_reblogs=true", account.id);
    let statuses_response = http_transport.execute(HttpRequest::get(statuses_url).header("Accept", "application/json")).await?;
    anyhow::ensure!(statuses_response.is_success(), "Mastodon statuses fetch failed: HTTP {}", statuses_response.status);
    let statuses: Vec<MastodonStatus> = serde_json::from_slice(&statuses_response.body).context("parsing Mastodon statuses response")?;

    statuses.into_iter().map(map_status).collect()
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
