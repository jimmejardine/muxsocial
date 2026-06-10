//! Per-network source clients. Each exposes a `fetch_recent_posts` that pulls a
//! handful of recent posts for a well-known identifier and maps them into the
//! normalized [`crate::post::AggregatedPost`].

pub mod bluesky;
pub mod hashiverse;
pub mod mastodon;
pub mod nostr;
