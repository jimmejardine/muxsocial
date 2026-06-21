//! Cross-posting (the write side): publish one composed message to every
//! authenticated account across the source networks.
//!
//! This is the write analogue of [`crate::timeline`]. Reading hides per-network
//! I/O behind the [`SourcePager`](crate::timeline::SourcePager) seam; writing
//! hides it behind [`SourcePoster`]. A [`SharedSourceWriters`] fans a single
//! [`ComposeRequest`] out to every authenticated account and collects one
//! [`PostResult`] per account (never short-circuiting, so one network's failure
//! still reports the others).
//!
//! Credentials live in [`account`] (an [`account::AuthenticatedAccount`] per
//! authenticated identity, supporting multiple accounts per network) and are
//! encrypted at rest via [`secret_box`].

pub mod account;
pub mod account_store;
pub mod network_poster;
pub mod oauth;
pub mod secret_box;
pub mod writers;

pub use account_store::AccountStore;
pub use network_poster::NetworkPoster;
pub use oauth::BeginOauthResult;
pub use writers::SharedSourceWriters;

use serde::Serialize;

use crate::post::SourceNetwork;

/// A composed message to broadcast. Text-only in v1 (media is future work).
#[derive(Debug, Clone)]
pub struct ComposeRequest {
    /// The post body as the user typed it (plain text).
    pub text: String,
    /// Creation time as Unix epoch millis. Bluesky needs an explicit `createdAt`
    /// (chrono has no `clock` on wasm), so the wasm bridge stamps it from
    /// `Date.now()`. Other networks set their own server-side time and ignore it.
    pub created_at_millis: i64,
}

impl ComposeRequest {
    /// A request with `created_at_millis` unset (0). The wasm bridge uses the
    /// struct literal with a real timestamp; this is for native/tests.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into(), created_at_millis: 0 }
    }
}

/// A reference to a post that was successfully published on a network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedPostReference {
    /// The network-native post id, when the publish call returns one.
    pub native_post_id: Option<String>,
    /// A canonical web permalink to the new post, when one can be built.
    pub post_url: Option<String>,
}

/// The per-account outcome of a cross-post, serialized for the GUI results panel
/// as a discriminated union on `status` (`published` / `failed`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PostOutcome {
    /// The post was published. Carries the permalink/id when available.
    Published { post_url: Option<String>, native_post_id: Option<String> },
    /// The post failed; `error_message` is shown to the user.
    Failed { error_message: String },
}

/// One row of the cross-post results: which account on which network, and how it
/// went. `account_label` disambiguates multiple accounts on the same network.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PostResult {
    pub network: SourceNetwork,
    pub account_label: String,
    pub outcome: PostOutcome,
}

impl PostResult {
    /// Build a result from a publish attempt, mapping `Ok`/`Err` into the
    /// serialized [`PostOutcome`]. Errors are rendered with the full anyhow chain.
    pub fn from_publish(network: SourceNetwork, account_label: impl Into<String>, publish_result: anyhow::Result<PublishedPostReference>) -> Self {
        let outcome = match publish_result {
            Ok(reference) => PostOutcome::Published {
                post_url: reference.post_url,
                native_post_id: reference.native_post_id,
            },
            Err(error) => PostOutcome::Failed { error_message: format!("{error:#}") },
        };
        Self {
            network,
            account_label: account_label.into(),
            outcome,
        }
    }
}

/// Per-network publish seam — the write analogue of
/// [`SourcePager`](crate::timeline::SourcePager). Implementors hold their bound
/// credentials/session and translate a [`ComposeRequest`] into a native publish.
///
/// Static dispatch only (the enum [`network_poster`] switches over concrete
/// posters), so the `async fn` form needs no `async-trait`/`Send` bound — same
/// reasoning as `SourcePager`.
#[allow(async_fn_in_trait)]
pub trait SourcePoster {
    /// Publish `request` to this poster's network using its bound credentials.
    async fn publish_post(&mut self, request: &ComposeRequest) -> anyhow::Result<PublishedPostReference>;
}

#[cfg(test)]
pub(crate) mod test_support {
    //! A deterministic, network-free poster for unit-testing the fan-out logic.

    use super::*;

    /// A poster that returns a canned outcome, ignoring the request.
    pub(crate) struct StubPoster {
        /// `Ok` publishes with this reference; `Err(message)` fails with it.
        pub(crate) outcome: Result<PublishedPostReference, String>,
    }

    impl SourcePoster for StubPoster {
        async fn publish_post(&mut self, _request: &ComposeRequest) -> anyhow::Result<PublishedPostReference> {
            match &self.outcome {
                Ok(reference) => Ok(reference.clone()),
                Err(message) => Err(anyhow::anyhow!(message.clone())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::StubPoster;
    use super::*;

    #[tokio::test]
    async fn fan_out_reports_one_result_per_account_including_failures() {
        // Two stub posters: one succeeds, one fails. A fan-out must surface both,
        // in order, without short-circuiting on the failure.
        let mut posters = vec![
            (
                SourceNetwork::Bluesky,
                "alice.bsky.social".to_string(),
                StubPoster {
                    outcome: Ok(PublishedPostReference {
                        native_post_id: Some("post-1".to_string()),
                        post_url: Some("https://bsky.app/profile/alice/post/1".to_string()),
                    }),
                },
            ),
            (
                SourceNetwork::Mastodon,
                "@bob@mastodon.social".to_string(),
                StubPoster {
                    outcome: Err("instance rejected the post".to_string()),
                },
            ),
        ];

        let request = ComposeRequest::new("hello world");
        let mut results: Vec<PostResult> = Vec::new();
        for (network, label, poster) in &mut posters {
            let publish_result = poster.publish_post(&request).await;
            results.push(PostResult::from_publish(*network, label.clone(), publish_result));
        }

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].network, SourceNetwork::Bluesky);
        assert!(matches!(results[0].outcome, PostOutcome::Published { .. }));
        assert_eq!(results[1].network, SourceNetwork::Mastodon);
        match &results[1].outcome {
            PostOutcome::Failed { error_message } => assert!(error_message.contains("instance rejected")),
            other => panic!("expected failure, got {other:?}"),
        }
    }
}
