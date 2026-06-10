//! A single source's pollable timeline.

use std::collections::HashSet;

use crate::post::AggregatedPost;
use crate::timeline::{Source, SourcePager, sort_posts_newest_first};

/// Manages the timeline for one [`Source`]: it accumulates a deduped,
/// newest-first list of posts and advances its position on each
/// [`SourceTimeline::get_more`].
///
/// `get_more` fetches the latest posts first; only if nothing newer arrives
/// does it page earlier than the oldest post held. Network specifics live in
/// the injected [`SourcePager`].
pub struct SourceTimeline<P: SourcePager> {
    source: Source,
    pager: P,
    /// All posts gathered so far, kept newest-first.
    posts: Vec<AggregatedPost>,
    /// Dedupe set keyed by `source_post_id` (unique within a single network).
    seen_post_ids: HashSet<String>,
    /// Set once an "earlier" fetch returns nothing new — there is nothing older.
    reached_oldest: bool,
}

impl<P: SourcePager> SourceTimeline<P> {
    pub fn new(source: Source, pager: P) -> Self {
        Self {
            source,
            pager,
            posts: Vec::new(),
            seen_post_ids: HashSet::new(),
            reached_oldest: false,
        }
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    /// The full accumulated timeline, newest-first.
    pub fn posts(&self) -> &[AggregatedPost] {
        &self.posts
    }

    /// True once paging earlier has stopped yielding new posts.
    pub fn reached_oldest(&self) -> bool {
        self.reached_oldest
    }

    /// Fetch the next chunk of posts. Tries the latest first (newer than the
    /// newest post held); if nothing newer is genuinely new, fetches earlier
    /// than the oldest post held. Returns only the posts newly added this call
    /// (use [`SourceTimeline::posts`] for the full ordered list).
    pub async fn get_more(&mut self, limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let newest_known = self.posts.first().cloned();
        let newer = self.pager.fetch_newer(newest_known.as_ref(), limit).await?;
        let added_newer = self.merge_new(newer);
        if !added_newer.is_empty() {
            return Ok(added_newer);
        }

        let oldest_known = self.posts.last().cloned();
        let older = self.pager.fetch_older(oldest_known.as_ref(), limit).await?;
        let added_older = self.merge_new(older);
        if added_older.is_empty() {
            self.reached_oldest = true;
        }
        Ok(added_older)
    }

    /// Clear the accumulated posts and restart paging at "now".
    pub async fn reset(&mut self) -> anyhow::Result<()> {
        self.pager.reset().await?;
        self.posts.clear();
        self.seen_post_ids.clear();
        self.reached_oldest = false;
        Ok(())
    }

    /// Merge fetched posts, dropping ones already seen. Returns the newly added
    /// posts (in fetched order) and keeps `posts` sorted newest-first.
    fn merge_new(&mut self, fetched: Vec<AggregatedPost>) -> Vec<AggregatedPost> {
        let mut added: Vec<AggregatedPost> = Vec::new();
        for post in fetched {
            if self.seen_post_ids.insert(post.source_post_id.clone()) {
                self.posts.push(post.clone());
                added.push(post);
            }
        }
        if !added.is_empty() {
            sort_posts_newest_first(&mut self.posts);
        }
        added
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post::SourceNetwork;
    use crate::timeline::test_support::{StubPager, test_post};

    fn nostr_timeline(posts: Vec<AggregatedPost>) -> SourceTimeline<StubPager> {
        SourceTimeline::new(Source::new(SourceNetwork::Nostr, "test-id"), StubPager::new(posts))
    }

    #[tokio::test]
    async fn first_get_more_returns_the_latest_page() {
        let posts = vec![test_post(SourceNetwork::Nostr, "a", 300), test_post(SourceNetwork::Nostr, "b", 200), test_post(SourceNetwork::Nostr, "c", 100)];
        let mut timeline = nostr_timeline(posts);

        let page = timeline.get_more(2).await.expect("get_more");
        let page_ids: Vec<&str> = page.iter().map(|post| post.source_post_id.as_str()).collect();
        assert_eq!(page_ids, vec!["a", "b"], "latest two newest-first");
        assert!(!timeline.reached_oldest());
    }

    #[tokio::test]
    async fn second_get_more_pages_earlier_when_no_newer() {
        let posts = vec![test_post(SourceNetwork::Nostr, "a", 300), test_post(SourceNetwork::Nostr, "b", 200), test_post(SourceNetwork::Nostr, "c", 100)];
        let mut timeline = nostr_timeline(posts);

        let first_page = timeline.get_more(2).await.expect("get_more");
        assert_eq!(first_page.len(), 2);

        // No newer posts exist, so this pulls earlier than the oldest held ("b" @ 200).
        let second_page = timeline.get_more(2).await.expect("get_more");
        let second_ids: Vec<&str> = second_page.iter().map(|post| post.source_post_id.as_str()).collect();
        assert_eq!(second_ids, vec!["c"], "older page");

        let all_ids: Vec<&str> = timeline.posts().iter().map(|post| post.source_post_id.as_str()).collect();
        assert_eq!(all_ids, vec!["a", "b", "c"], "full list stays newest-first");
    }

    #[tokio::test]
    async fn reaches_oldest_when_nothing_earlier_remains() {
        let posts = vec![test_post(SourceNetwork::Nostr, "a", 200), test_post(SourceNetwork::Nostr, "b", 100)];
        let mut timeline = nostr_timeline(posts);

        let _ = timeline.get_more(5).await.expect("get_more"); // grabs both
        let empty = timeline.get_more(5).await.expect("get_more"); // nothing newer, nothing older
        assert!(empty.is_empty());
        assert!(timeline.reached_oldest());
    }

    #[tokio::test]
    async fn a_newly_published_post_surfaces_ahead_of_older_paging() {
        let mut all = vec![test_post(SourceNetwork::Nostr, "a", 300), test_post(SourceNetwork::Nostr, "b", 200), test_post(SourceNetwork::Nostr, "c", 100)];
        let mut timeline = SourceTimeline::new(Source::new(SourceNetwork::Nostr, "test-id"), StubPager::new(all.clone()));

        let _ = timeline.get_more(5).await.expect("get_more"); // a, b, c

        // A brand-new post appears at the top of the source.
        all.push(test_post(SourceNetwork::Nostr, "d", 400));
        timeline.pager_mut().all_posts_newest_first = {
            let mut sorted = all.clone();
            crate::timeline::sort_posts_newest_first(&mut sorted);
            sorted
        };

        let page = timeline.get_more(5).await.expect("get_more");
        let page_ids: Vec<&str> = page.iter().map(|post| post.source_post_id.as_str()).collect();
        assert_eq!(page_ids, vec!["d"], "the new latest post is returned, not older paging");
        assert_eq!(timeline.posts().first().map(|post| post.source_post_id.as_str()), Some("d"));
    }

    #[tokio::test]
    async fn duplicates_are_not_added_twice() {
        let posts = vec![test_post(SourceNetwork::Nostr, "a", 200), test_post(SourceNetwork::Nostr, "b", 100)];
        let mut timeline = nostr_timeline(posts);

        let _ = timeline.get_more(5).await.expect("get_more");
        assert_eq!(timeline.posts().len(), 2);
        // Re-running cannot duplicate the already-seen posts.
        let _ = timeline.get_more(5).await.expect("get_more");
        assert_eq!(timeline.posts().len(), 2);
    }

    impl<P: SourcePager> SourceTimeline<P> {
        /// Test-only access to the underlying pager.
        fn pager_mut(&mut self) -> &mut P {
            &mut self.pager
        }
    }
}
