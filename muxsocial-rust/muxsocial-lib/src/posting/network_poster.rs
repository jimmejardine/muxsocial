//! Static dispatch over the per-network posters — the write analogue of
//! [`NetworkPager`](crate::timeline::NetworkPager).
//!
//! [`SharedSourceWriters`](crate::posting::writers::SharedSourceWriters) builds
//! one of these per authenticated account and calls
//! [`SourcePoster::publish_post`] on it. Mastodon and Bluesky variants are added
//! in their own stages; until then, building a poster for those networks errors
//! (surfaced as a per-account failure, never a panic).

use crate::posting::{ComposeRequest, PublishedPostReference, SourcePoster};
use crate::sources::hashiverse::HashiversePoster;
use crate::sources::mastodon::MastodonPoster;
use crate::sources::nostr::{KeysEventSigner, NostrPoster};

/// A concrete poster for one network, dispatched by [`SourcePoster`].
pub enum NetworkPoster {
    /// nostr, signing with a pasted-nsec key (the [`KeysEventSigner`] seam also
    /// admits a future NIP-07 signer as a separate variant).
    Nostr(NostrPoster<KeysEventSigner>),
    /// Hashiverse, over an authenticated (keyphrase-unlocked) client.
    Hashiverse(HashiversePoster),
    /// Mastodon, over an OAuth bearer token.
    Mastodon(MastodonPoster),
}

impl SourcePoster for NetworkPoster {
    async fn publish_post(&mut self, request: &ComposeRequest) -> anyhow::Result<PublishedPostReference> {
        match self {
            NetworkPoster::Nostr(poster) => poster.publish_post(request).await,
            NetworkPoster::Hashiverse(poster) => poster.publish_post(request).await,
            NetworkPoster::Mastodon(poster) => poster.publish_post(request).await,
        }
    }
}
