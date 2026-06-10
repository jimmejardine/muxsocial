use clap::Parser;
use hashiverse_client_rust::HashiverseBuilder;
use muxsocial_lib::greeting::compose_greeting_message;
use muxsocial_lib::http::default_http_transport;
use muxsocial_lib::post::{AggregatedPost, SourceNetwork};
use muxsocial_lib::sources::bluesky::BlueskyPager;
use muxsocial_lib::sources::hashiverse::HashiversePager;
use muxsocial_lib::sources::mastodon::MastodonPager;
use muxsocial_lib::sources::nostr::{self, NostrPager};
use muxsocial_lib::timeline::{MultiTimeline, NetworkPager, Source, SourceTimeline};
use std::io::{self, BufRead, Write};
use std::time::Duration;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[derive(Parser, Debug, Clone)]
#[command(name = "test-harness", about = "mux.social long-running integration test harness")]
struct TestHarnessArguments {
    /// Recipient name used by the `t` greeting command
    #[arg(long, default_value = "integration-harness")]
    recipient_name: String,
    /// Base log level for the native logging listener (overridable per-module via RUST_LOG)
    #[arg(long, default_value = "trace")]
    log_level: String,
}

/// Initialise the native logging listener so `log::` records from muxsocial-lib
/// and the source SDKs surface (tracing-subscriber's default `tracing-log`
/// bridge captures the `log` facade). `RUST_LOG` overrides the default filter;
/// otherwise noisy infra crates are silenced so muxsocial and the network SDKs
/// stay readable. The GUI logs the same `log` events via `wasm_init` instead.
fn configure_logging_listener(level: &str) {
    let social_clients_at_warn: &str = "nostr=warn,nostr_sdk=warn,nostr_relay_pool=warn,nostr_database=warn,nostr_gossip=warn,async_wsocket=warn,atrium_api=warn,atrium_common=warn,atrium_xrpc=warn,hashiverse_lib=warn,hashiverse_client_rust=warn";
    let noisy_infra_off: &str = "hyper=off,reqwest=off,rustls=off,h2=off,hickory_resolver=off,hickory_proto=off,tokio_tungstenite=off,tungstenite=off,mio=off,want=off";
    let default_filter: String = format!("{level},{social_clients_at_warn},{noisy_infra_off}");
    let env_filter: EnvFilter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));

    tracing_subscriber::registry().with(fmt::layer()).with(env_filter).init();

    log::info!("Logging initialized");
}

/// A source network the harness can pull a sample feed from.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum NetworkChoice {
    Nostr,
    Bluesky,
    Mastodon,
    Hashiverse,
}

/// Outcome of analysing one typed sentence in the harness REPL.
#[derive(Debug, PartialEq, Eq)]
enum SentenceDispatchOutcome {
    /// A subsystem handled the sentence and produced output to display.
    Handled(String),
    /// Page a single source network's timeline (handled asynchronously in `main`).
    PageNetwork(NetworkChoice),
    /// Page the mixed MultiTimeline (nostr + Bluesky + Mastodon).
    PageMixed,
    /// The sentence did not match any known command.
    Unrecognised(String),
    /// The user asked to quit the harness.
    Quit,
}

/// Analyse a typed sentence and route it to the correct subsystem.
/// `t` composes the greeting; `tn`/`tb`/`tm`/`th` page nostr, Bluesky, Mastodon,
/// and Hashiverse timelines (repeat to fetch further); `tx` pages the mixed
/// MultiTimeline. Each uses a baked-in default identifier.
fn analyse_and_dispatch_sentence(sentence: &str, recipient_name: &str) -> SentenceDispatchOutcome {
    let trimmed_sentence: &str = sentence.trim();
    match trimmed_sentence {
        "q" | "quit" | "exit" => SentenceDispatchOutcome::Quit,
        "t" => SentenceDispatchOutcome::Handled(compose_greeting_message(recipient_name)),
        "tn" => SentenceDispatchOutcome::PageNetwork(NetworkChoice::Nostr),
        "tb" => SentenceDispatchOutcome::PageNetwork(NetworkChoice::Bluesky),
        "tm" => SentenceDispatchOutcome::PageNetwork(NetworkChoice::Mastodon),
        "th" => SentenceDispatchOutcome::PageNetwork(NetworkChoice::Hashiverse),
        "tx" => SentenceDispatchOutcome::PageMixed,
        _ => SentenceDispatchOutcome::Unrecognised(trimmed_sentence.to_string()),
    }
}

/// Well-known identifiers used for the harness sample feeds.
const NOSTR_AUTHOR_NPUB: &str = "npub1wmr34t36fy03m8hvgl96zl3znndyzyaqhwmwdtshwmtkg03fetaqhjg240";
const BLUESKY_ACTOR: &str = "jay.bsky.team";
const MASTODON_ACCOUNT: &str = "@Gargron@mastodon.social";
const HASHIVERSE_USER_ID: &str = "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0";

/// Posts fetched per `get_more` call.
const PAGE_LIMIT: usize = 5;

fn nostr_relay_timeout() -> Duration {
    Duration::from_secs(20)
}

/// Persistent, lazily-constructed timelines so repeated commands page further
/// rather than re-fetching the top. The mixed timeline omits Hashiverse so it
/// doesn't open a second sqlite-backed client alongside the standalone `th`.
#[derive(Default)]
struct HarnessTimelines {
    nostr: Option<SourceTimeline<NetworkPager>>,
    bluesky: Option<SourceTimeline<NetworkPager>>,
    mastodon: Option<SourceTimeline<NetworkPager>>,
    hashiverse: Option<SourceTimeline<NetworkPager>>,
    mixed: Option<MultiTimeline<NetworkPager>>,
}

impl HarnessTimelines {
    fn nostr_source_timeline() -> SourceTimeline<NetworkPager> {
        SourceTimeline::new(
            Source::new(SourceNetwork::Nostr, NOSTR_AUTHOR_NPUB),
            NetworkPager::Nostr(NostrPager::new(NOSTR_AUTHOR_NPUB, nostr::DEFAULT_RELAYS, nostr_relay_timeout())),
        )
    }

    fn bluesky_source_timeline() -> SourceTimeline<NetworkPager> {
        SourceTimeline::new(Source::new(SourceNetwork::Bluesky, BLUESKY_ACTOR), NetworkPager::Bluesky(BlueskyPager::new(default_http_transport(), BLUESKY_ACTOR)))
    }

    fn mastodon_source_timeline() -> SourceTimeline<NetworkPager> {
        SourceTimeline::new(Source::new(SourceNetwork::Mastodon, MASTODON_ACCOUNT), NetworkPager::Mastodon(MastodonPager::new(default_http_transport(), MASTODON_ACCOUNT)))
    }

    fn nostr_timeline(&mut self) -> &mut SourceTimeline<NetworkPager> {
        self.nostr.get_or_insert_with(Self::nostr_source_timeline)
    }

    fn bluesky_timeline(&mut self) -> &mut SourceTimeline<NetworkPager> {
        self.bluesky.get_or_insert_with(Self::bluesky_source_timeline)
    }

    fn mastodon_timeline(&mut self) -> &mut SourceTimeline<NetworkPager> {
        self.mastodon.get_or_insert_with(Self::mastodon_source_timeline)
    }

    /// Build (once) and return the Hashiverse timeline. Construction is async and
    /// slow (it spins up a native DHT client), so it's deferred to first use.
    async fn hashiverse_timeline(&mut self) -> anyhow::Result<&mut SourceTimeline<NetworkPager>> {
        if self.hashiverse.is_none() {
            let user_id_hex = std::env::var("MUXSOCIAL_HASHIVERSE_TEST_USER_ID").unwrap_or_else(|_| HASHIVERSE_USER_ID.to_string());
            let hashiverse = HashiverseBuilder::new().data_dir(std::env::temp_dir().join("muxsocial-hashiverse-harness")).build_with_keyphrase("muxsocial-harness").await?;
            let hashiverse_pager = HashiversePager::new(hashiverse.client().clone(), user_id_hex.clone());
            self.hashiverse = Some(SourceTimeline::new(Source::new(SourceNetwork::Hashiverse, user_id_hex), NetworkPager::Hashiverse(hashiverse_pager)));
        }
        Ok(self.hashiverse.as_mut().expect("hashiverse timeline just built"))
    }

    fn mixed_timeline(&mut self) -> &mut MultiTimeline<NetworkPager> {
        self.mixed
            .get_or_insert_with(|| MultiTimeline::new(vec![Self::nostr_source_timeline(), Self::bluesky_source_timeline(), Self::mastodon_source_timeline()]))
    }
}

/// Page a single-source timeline and render the newly-fetched posts.
async fn page_and_render(timeline: &mut SourceTimeline<NetworkPager>) -> String {
    match timeline.get_more(PAGE_LIMIT).await {
        Ok(new_posts) => render_timeline_page(timeline.source(), &new_posts, timeline.posts().len(), timeline.reached_oldest()),
        Err(fetch_error) => format!("fetch failed: {fetch_error:#}"),
    }
}

fn render_timeline_page(source: &Source, new_posts: &[AggregatedPost], total_held: usize, reached_oldest: bool) -> String {
    let oldest_note = if reached_oldest { " (reached oldest)" } else { "" };
    let mut rendered = format!("{:?} [{}]: +{} new, {total_held} held{oldest_note}\n", source.network, source.id, new_posts.len());
    for post in new_posts {
        rendered.push_str(&render_post_line(post, false));
    }
    rendered
}

fn render_post_line(post: &AggregatedPost, show_network: bool) -> String {
    let author = post.author_display_name.as_deref().unwrap_or(&post.author_identifier);
    let preview: String = post.content_html.chars().take(160).collect();
    let network_tag = if show_network { format!("{:?} ", post.source) } else { String::new() };
    format!("  [{}] {network_tag}{author}: {preview}\n", post.created_at_millis)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let test_harness_arguments: TestHarnessArguments = TestHarnessArguments::parse();
    configure_logging_listener(&test_harness_arguments.log_level);

    let mut harness_timelines = HarnessTimelines::default();

    let standard_input = io::stdin();
    let mut standard_output = io::stdout();
    let mut line_buffer: String = String::new();

    loop {
        write!(standard_output, "> ")?;
        standard_output.flush()?;

        line_buffer.clear();
        let bytes_read: usize = standard_input.lock().read_line(&mut line_buffer)?;
        if bytes_read == 0 {
            // End of input (Ctrl+Z on Windows / Ctrl+D elsewhere) closes the harness.
            writeln!(standard_output)?;
            break;
        }

        match analyse_and_dispatch_sentence(&line_buffer, &test_harness_arguments.recipient_name) {
            SentenceDispatchOutcome::Handled(output_message) => {
                println!("{output_message}");
            }
            SentenceDispatchOutcome::PageNetwork(network_choice) => {
                let rendered = match network_choice {
                    NetworkChoice::Nostr => page_and_render(harness_timelines.nostr_timeline()).await,
                    NetworkChoice::Bluesky => page_and_render(harness_timelines.bluesky_timeline()).await,
                    NetworkChoice::Mastodon => page_and_render(harness_timelines.mastodon_timeline()).await,
                    NetworkChoice::Hashiverse => match harness_timelines.hashiverse_timeline().await {
                        Ok(timeline) => page_and_render(timeline).await,
                        Err(setup_error) => format!("Hashiverse setup failed: {setup_error:#}"),
                    },
                };
                println!("{rendered}");
            }
            SentenceDispatchOutcome::PageMixed => {
                let mixed_timeline = harness_timelines.mixed_timeline();
                let rendered = match mixed_timeline.get_more(PAGE_LIMIT).await {
                    Ok(new_posts) => {
                        let mut out = format!("Mixed: +{} new, {} held\n", new_posts.len(), mixed_timeline.posts().len());
                        for post in &new_posts {
                            out.push_str(&render_post_line(post, true));
                        }
                        out
                    }
                    Err(fetch_error) => format!("mixed fetch failed: {fetch_error:#}"),
                };
                println!("{rendered}");
            }
            SentenceDispatchOutcome::Unrecognised(sentence) => {
                println!("Unrecognised command: \"{sentence}\"");
            }
            SentenceDispatchOutcome::Quit => {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_dispatches_to_the_greeting_message() {
        let dispatch_outcome: SentenceDispatchOutcome = analyse_and_dispatch_sentence("t", "harness-recipient");
        assert_eq!(dispatch_outcome, SentenceDispatchOutcome::Handled(compose_greeting_message("harness-recipient")));
    }

    #[test]
    fn surrounding_whitespace_around_t_still_dispatches() {
        let dispatch_outcome: SentenceDispatchOutcome = analyse_and_dispatch_sentence("  t\n", "harness-recipient");
        assert_eq!(dispatch_outcome, SentenceDispatchOutcome::Handled(compose_greeting_message("harness-recipient")));
    }

    #[test]
    fn network_commands_route_to_their_networks() {
        for (command, expected_network) in [("tn", NetworkChoice::Nostr), ("tb", NetworkChoice::Bluesky), ("tm", NetworkChoice::Mastodon), ("th", NetworkChoice::Hashiverse)] {
            let dispatch_outcome = analyse_and_dispatch_sentence(command, "harness-recipient");
            assert_eq!(dispatch_outcome, SentenceDispatchOutcome::PageNetwork(expected_network), "expected {command:?} to route to {expected_network:?}");
        }
    }

    #[test]
    fn tx_routes_to_the_mixed_timeline() {
        let dispatch_outcome = analyse_and_dispatch_sentence("tx", "harness-recipient");
        assert_eq!(dispatch_outcome, SentenceDispatchOutcome::PageMixed);
    }

    #[test]
    fn quit_keywords_request_quit() {
        for quit_keyword in ["q", "quit", "exit"] {
            let dispatch_outcome: SentenceDispatchOutcome = analyse_and_dispatch_sentence(quit_keyword, "harness-recipient");
            assert_eq!(dispatch_outcome, SentenceDispatchOutcome::Quit, "expected {quit_keyword:?} to request quit");
        }
    }

    #[test]
    fn unknown_sentence_is_unrecognised_with_trimmed_text() {
        let dispatch_outcome: SentenceDispatchOutcome = analyse_and_dispatch_sentence("  hello world \n", "harness-recipient");
        assert_eq!(dispatch_outcome, SentenceDispatchOutcome::Unrecognised("hello world".to_string()));
    }

    #[test]
    fn empty_sentence_is_unrecognised() {
        let dispatch_outcome: SentenceDispatchOutcome = analyse_and_dispatch_sentence("   \n", "harness-recipient");
        assert_eq!(dispatch_outcome, SentenceDispatchOutcome::Unrecognised(String::new()));
    }
}
