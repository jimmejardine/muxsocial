//! A single concrete [`SourcePager`] covering all networks, so heterogeneous
//! sources can share one `SourceTimeline<NetworkPager>` / `MultiTimeline<NetworkPager>`
//! type. (Tests use a stub pager instead — see `timeline::test_support`.)

use crate::post::AggregatedPost;
use crate::sources::bluesky::BlueskyPager;
use crate::sources::hashiverse::HashiversePager;
use crate::sources::mastodon::MastodonPager;
use crate::sources::nostr::NostrPager;
use crate::timeline::SourcePager;

/// Enum-dispatched pager over the four source networks.
// One pager is constructed per source and lives inside a `SourceTimeline`; it's
// never moved on a hot path, so the size spread between variants doesn't matter.
#[allow(clippy::large_enum_variant)]
pub enum NetworkPager {
    Nostr(NostrPager),
    Bluesky(BlueskyPager),
    Mastodon(MastodonPager),
    Hashiverse(HashiversePager),
}

impl SourcePager for NetworkPager {
    async fn fetch_newer(&mut self, newest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        match self {
            NetworkPager::Nostr(pager) => pager.fetch_newer(newest_known, limit).await,
            NetworkPager::Bluesky(pager) => pager.fetch_newer(newest_known, limit).await,
            NetworkPager::Mastodon(pager) => pager.fetch_newer(newest_known, limit).await,
            NetworkPager::Hashiverse(pager) => pager.fetch_newer(newest_known, limit).await,
        }
    }

    async fn fetch_older(&mut self, oldest_known: Option<&AggregatedPost>, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        match self {
            NetworkPager::Nostr(pager) => pager.fetch_older(oldest_known, limit).await,
            NetworkPager::Bluesky(pager) => pager.fetch_older(oldest_known, limit).await,
            NetworkPager::Mastodon(pager) => pager.fetch_older(oldest_known, limit).await,
            NetworkPager::Hashiverse(pager) => pager.fetch_older(oldest_known, limit).await,
        }
    }

    async fn reset(&mut self) -> anyhow::Result<()> {
        match self {
            NetworkPager::Nostr(pager) => pager.reset().await,
            NetworkPager::Bluesky(pager) => pager.reset().await,
            NetworkPager::Mastodon(pager) => pager.reset().await,
            NetworkPager::Hashiverse(pager) => pager.reset().await,
        }
    }
}
