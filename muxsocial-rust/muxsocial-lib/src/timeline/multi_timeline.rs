//! A timeline that mixes several [`SourceTimeline`]s into one stream.
//!
//! Mirrors hashiverse's `MultipleTimeline`: it holds several single-source
//! timelines and, on each `get_more`, pulls from each and merges the results
//! into one newest-first list. (The basic version here pulls from every child
//! each call; hashiverse's penalty-quota scheduling is a later refinement.)

use crate::post::AggregatedPost;
use crate::timeline::{Source, SourcePager, SourceTimeline, sort_posts_newest_first};

pub struct MultiTimeline<P: SourcePager> {
    children: Vec<SourceTimeline<P>>,
    /// All children merged, newest-first.
    merged: Vec<AggregatedPost>,
}

impl<P: SourcePager> MultiTimeline<P> {
    pub fn new(children: Vec<SourceTimeline<P>>) -> Self {
        Self { children, merged: Vec::new() }
    }

    /// The sources currently mixed into this timeline.
    pub fn sources(&self) -> Vec<&Source> {
        self.children.iter().map(|child| child.source()).collect()
    }

    /// The full merged timeline, newest-first.
    pub fn posts(&self) -> &[AggregatedPost] {
        &self.merged
    }

    /// True once every child has reached its oldest.
    pub fn reached_oldest(&self) -> bool {
        !self.children.is_empty() && self.children.iter().all(|child| child.reached_oldest())
    }

    /// Pull more posts from each child source and merge. Returns the posts newly
    /// added across all sources this call (use [`MultiTimeline::posts`] for the
    /// full ordered list). Cross-source duplicates cannot occur (distinct
    /// networks), so per-child dedupe is sufficient.
    pub async fn get_more(&mut self, per_source_limit: usize) -> anyhow::Result<Vec<AggregatedPost>> {
        let mut added_all: Vec<AggregatedPost> = Vec::new();
        for child in &mut self.children {
            let added = child.get_more(per_source_limit).await?;
            added_all.extend(added);
        }
        if !added_all.is_empty() {
            self.merged.extend(added_all.iter().cloned());
            sort_posts_newest_first(&mut self.merged);
            // Return the page newest-first too, interleaved across sources.
            sort_posts_newest_first(&mut added_all);
        }
        Ok(added_all)
    }

    /// Reset every child and clear the merged list.
    pub async fn reset(&mut self) -> anyhow::Result<()> {
        for child in &mut self.children {
            child.reset().await?;
        }
        self.merged.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::post::SourceNetwork;
    use crate::timeline::test_support::{StubPager, test_post};

    #[tokio::test]
    async fn interleaves_sources_newest_first() {
        let nostr = SourceTimeline::new(Source::new(SourceNetwork::Nostr, "n"), StubPager::new(vec![test_post(SourceNetwork::Nostr, "n1", 400), test_post(SourceNetwork::Nostr, "n2", 100)]));
        let mastodon = SourceTimeline::new(
            Source::new(SourceNetwork::Mastodon, "m"),
            StubPager::new(vec![test_post(SourceNetwork::Mastodon, "m1", 300), test_post(SourceNetwork::Mastodon, "m2", 200)]),
        );
        let mut multi = MultiTimeline::new(vec![nostr, mastodon]);

        let page = multi.get_more(5).await.expect("get_more");
        assert_eq!(page.len(), 4, "all four posts pulled across both sources");

        let merged_ids: Vec<&str> = multi.posts().iter().map(|post| post.source_post_id.as_str()).collect();
        assert_eq!(merged_ids, vec!["n1", "m1", "m2", "n2"], "merged newest-first across networks");
    }

    #[tokio::test]
    async fn reports_reached_oldest_only_when_all_children_exhausted() {
        let nostr = SourceTimeline::new(Source::new(SourceNetwork::Nostr, "n"), StubPager::new(vec![test_post(SourceNetwork::Nostr, "n1", 100)]));
        let mastodon = SourceTimeline::new(Source::new(SourceNetwork::Mastodon, "m"), StubPager::new(vec![test_post(SourceNetwork::Mastodon, "m1", 200)]));
        let mut multi = MultiTimeline::new(vec![nostr, mastodon]);

        let _ = multi.get_more(5).await.expect("get_more"); // pulls n1, m1
        assert!(!multi.reached_oldest());
        let empty = multi.get_more(5).await.expect("get_more"); // nothing newer/older anywhere
        assert!(empty.is_empty());
        assert!(multi.reached_oldest());
    }
}
