//! Live Hashiverse smoke test (built natively via the locally-patched moka).
//!
//! Reading the live network needs a real user id; set
//! `MUXSOCIAL_HASHIVERSE_TEST_USER_ID` to a 64-char hex id to run it. Without
//! that env var the test compiles and passes as a skip, since there is no
//! well-known/founder id baked into hashiverse-lib.

use hashiverse_client_rust::HashiverseBuilder;
use muxsocial_lib::post::SourceNetwork;
use muxsocial_lib::sources::hashiverse;

#[tokio::test]
async fn pulls_recent_posts_from_hashiverse() {
    let Ok(user_id_hex) = std::env::var("MUXSOCIAL_HASHIVERSE_TEST_USER_ID")
    else {
        eprintln!("skipping hashiverse pull test: set MUXSOCIAL_HASHIVERSE_TEST_USER_ID to a hex user id to run it");
        return;
    };

    // Sensible native defaults: sqlite storage + on-disk key locker under a temp
    // dir, DNSSEC bootstrap, native parallel PoW. The keyphrase just derives the
    // reader's own identity; it is irrelevant to whose timeline we read.
    let hashiverse_client = HashiverseBuilder::new()
        .data_dir(std::env::temp_dir().join("muxsocial-hashiverse-integration-test"))
        .build_with_keyphrase("muxsocial-integration-test")
        .await
        .expect("building hashiverse client should succeed");

    let posts = hashiverse::fetch_recent_posts(&hashiverse_client, &user_id_hex).await.expect("hashiverse fetch should succeed");

    assert!(!posts.is_empty(), "expected at least one hashiverse post for {user_id_hex}");
    for post in &posts {
        assert_eq!(post.source, SourceNetwork::Hashiverse);
        assert!(!post.source_post_id.is_empty(), "post should have an id");
        assert_eq!(post.author_identifier, user_id_hex, "post should be attributed to the requested user id");
    }
}
