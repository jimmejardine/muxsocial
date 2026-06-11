//! The normalized post type that every source network maps into.
//!
//! This is deliberately small — it is the lowest common denominator across
//! Hashiverse, nostr, Mastodon, and Bluesky that is enough to prove we can pull
//! and display a feed. Richer, versioned wire/bundle types come later and are
//! specced separately.

use serde::{Deserialize, Serialize};

/// Which source network a post was aggregated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceNetwork {
    Hashiverse,
    Nostr,
    Mastodon,
    Bluesky,
}

impl SourceNetwork {
    /// Parse the serialized variant name (`"Hashiverse"`, `"Nostr"`, …) back into
    /// the enum — the form the GUI sends across the wasm boundary.
    pub fn from_name(name: &str) -> Option<SourceNetwork> {
        match name {
            "Hashiverse" => Some(SourceNetwork::Hashiverse),
            "Nostr" => Some(SourceNetwork::Nostr),
            "Mastodon" => Some(SourceNetwork::Mastodon),
            "Bluesky" => Some(SourceNetwork::Bluesky),
            _ => None,
        }
    }
}

/// A single post normalized across all source networks.
///
/// `content_html` is HTML for every source: Mastodon and Hashiverse bodies are
/// HTML natively; Bluesky is rendered from its text + richtext facets; nostr's
/// plain text is escaped and wrapped (newlines → `<br>`). Sanitization/rendering
/// is a later concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregatedPost {
    /// The network this post came from.
    pub source: SourceNetwork,
    /// The network-native, stable identifier of the post (hex id, AT-URI, event id, status id).
    pub source_post_id: String,
    /// The network-native author identifier (pubkey hex, handle, acct, DID).
    pub author_identifier: String,
    /// A human-friendly author name when the network exposes one cheaply.
    pub author_display_name: Option<String>,
    /// Post creation time as Unix epoch milliseconds (UTC).
    pub created_at_millis: i64,
    /// The post body as HTML (see the type-level note).
    pub content_html: String,
    /// A canonical web permalink to the original post on its network, when one can
    /// be built (njump for nostr, bsky.app for Bluesky, the status URL for
    /// Mastodon, the Hashiverse app route). `None` when no link is available.
    pub post_url: Option<String>,
    /// Structured media attached to the post (images, video, link cards),
    /// rendered by the GUI below the body. Empty when the source carries no
    /// separate media (Hashiverse keeps any media inline in `content_html`).
    #[serde(default)]
    pub media: Vec<PostMedia>,
}

/// A piece of media attached to a post. Serialized internally-tagged on `kind`
/// (`image` / `video` / `link_card`) so the GUI can switch on it as a
/// discriminated union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PostMedia {
    /// A still image (or animated GIF rendered as one).
    Image { url: String, alt: Option<String> },
    /// A video; `poster` is a still frame/thumbnail to show before playback.
    Video { url: String, poster: Option<String>, alt: Option<String> },
    /// An external link preview card.
    LinkCard {
        url: String,
        title: Option<String>,
        description: Option<String>,
        thumbnail_url: Option<String>,
    },
}

/// Parse an RFC3339 / ISO-8601 timestamp (as emitted by Mastodon and Bluesky)
/// into Unix epoch milliseconds.
pub fn parse_rfc3339_to_epoch_millis(timestamp_text: &str) -> anyhow::Result<i64> {
    let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(timestamp_text).map_err(|parse_error| anyhow::anyhow!("invalid RFC3339 timestamp {timestamp_text:?}: {parse_error}"))?;
    Ok(parsed_timestamp.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_timestamp_to_epoch_millis() {
        // 2021-01-01T00:00:00Z is 1609459200 seconds since the epoch.
        let epoch_millis = parse_rfc3339_to_epoch_millis("2021-01-01T00:00:00.000Z").expect("should parse");
        assert_eq!(epoch_millis, 1_609_459_200_000);
    }

    #[test]
    fn parses_rfc3339_timestamp_with_offset() {
        let epoch_millis = parse_rfc3339_to_epoch_millis("2021-01-01T01:00:00+01:00").expect("should parse");
        assert_eq!(epoch_millis, 1_609_459_200_000);
    }

    #[test]
    fn rejects_non_timestamp_text() {
        assert!(parse_rfc3339_to_epoch_millis("not-a-date").is_err());
    }
}
