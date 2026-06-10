//! Stateful, pollable timelines over the source networks.
//!
//! This mirrors hashiverse's `SingleTimeline`/`MultipleTimeline`
//! (`hashiverse-lib/src/client/timeline/`): a [`SourceTimeline`] owns the
//! merged/deduped/ordered post list for one [`Source`] and advances its
//! position on each [`SourceTimeline::get_more`]; a [`MultiTimeline`] mixes
//! several together.
//!
//! Each `get_more` fetches the **latest** posts first (newer than what we
//! already hold); if nothing newer arrives it fetches some posts **earlier**
//! than the oldest we hold. The GUI calls `get_more` when the scroll area
//! reaches the bottom.
//!
//! The per-network I/O is hidden behind the [`SourcePager`] seam — the analogue
//! of hashiverse's injected `PostBundleManager`. Each network paginates
//! differently (nostr by timestamp, Mastodon by status id, Bluesky by opaque
//! cursor, Hashiverse via its own `SingleTimeline`), so each provides its own
//! pager; see [`crate::sources`] and [`network_pager::NetworkPager`].

pub mod multi_timeline;
pub mod network_pager;
pub mod source_timeline;

pub use multi_timeline::MultiTimeline;
pub use network_pager::NetworkPager;
pub use source_timeline::SourceTimeline;

use serde::{Deserialize, Serialize};

use crate::post::{AggregatedPost, SourceNetwork};

/// A timeline source: a network plus its network-native identifier (npub/hex
/// for nostr, an actor handle/DID for Bluesky, `@user@instance` for Mastodon, a
/// user-id hex for Hashiverse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub network: SourceNetwork,
    pub id: String,
}

impl Source {
    pub fn new(network: SourceNetwork, id: impl Into<String>) -> Self {
        Self { network, id: id.into() }
    }
}

/// Per-network pagination seam. Implementors translate "newer/older than the
/// posts we already hold" into each network's native query. `&mut self` lets a
/// pager keep network-specific state (e.g. Bluesky's opaque older-cursor).
///
/// Used only with static dispatch (the timelines are generic over `P`), so the
/// `async fn` form is fine and no `async-trait`/`Send` bound is needed.
#[allow(async_fn_in_trait)]
pub trait SourcePager {
    /// Fetch posts newer than `newest_known` (None = initial "latest" fetch).
    async fn fetch_newer(&mut self, newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>>;
    /// Fetch posts older than `oldest_known` (None = initial fetch).
    async fn fetch_older(&mut self, oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>>;
    /// Restart paging at "now" (Hashiverse → `single_timeline_reset`; others → clear cursors).
    async fn reset(&mut self) -> anyhow::Result<()>;
}

/// Sort posts newest-first, tie-breaking on `source_post_id` for a stable order.
pub(crate) fn sort_posts_newest_first(posts: &mut [AggregatedPost]) {
    posts.sort_by(|a, b| b.created_at_millis.cmp(&a.created_at_millis).then_with(|| a.source_post_id.cmp(&b.source_post_id)));
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Deterministic, network-free pager for unit-testing the timeline state
    //! machines.

    use super::*;

    /// Build a throwaway [`AggregatedPost`] for tests.
    pub(crate) fn test_post(network: SourceNetwork, source_post_id: &str, created_at_millis: i64) -> AggregatedPost {
        AggregatedPost {
            source: network,
            source_post_id: source_post_id.to_string(),
            author_identifier: "test-author".to_string(),
            author_display_name: None,
            created_at_millis,
            content_html: format!("<p>{source_post_id}</p>"),
        }
    }

    /// A pager backed by a fixed in-memory post set (newest-first). It serves
    /// `fetch_newer`/`fetch_older` purely from timestamps, so the timeline logic
    /// can be exercised without any network.
    pub(crate) struct StubPager {
        /// The full "server" content, newest-first.
        pub all_posts_newest_first: Vec<AggregatedPost>,
    }

    impl StubPager {
        pub(crate) fn new(mut all_posts: Vec<AggregatedPost>) -> Self {
            sort_posts_newest_first(&mut all_posts);
            Self { all_posts_newest_first: all_posts }
        }

        fn top(&self, limit: usize) -> Vec<AggregatedPost> {
            self.all_posts_newest_first.iter().take(limit).cloned().collect()
        }
    }

    impl SourcePager for StubPager {
        async fn fetch_newer(&mut self, newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
            Ok(match newest_known {
                None => self.top(limit),
                Some(newest) => self.all_posts_newest_first.iter().filter(|post| post.created_at_millis > newest.created_at_millis).take(limit).cloned().collect(),
            })
        }

        async fn fetch_older(&mut self, oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
            Ok(match oldest_known {
                None => self.top(limit),
                // `all_posts_newest_first` is newest-first, so the older posts come
                // later in the vec; filtering `< oldest` and taking `limit` yields
                // the `limit` newest of the older posts.
                Some(oldest) => self.all_posts_newest_first.iter().filter(|post| post.created_at_millis < oldest.created_at_millis).take(limit).cloned().collect(),
            })
        }

        async fn reset(&mut self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
