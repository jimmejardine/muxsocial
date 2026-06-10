use clap::Parser;
use hashiverse_client_rust::HashiverseBuilder;
use muxsocial_lib::greeting::compose_greeting_message;
use muxsocial_lib::http::default_http_transport;
use muxsocial_lib::post::AggregatedPost;
use muxsocial_lib::sources::{bluesky, hashiverse, mastodon, nostr};
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
    let default_filter: String = format!("{level},hyper=off,reqwest=off,rustls=off,h2=off,hickory_resolver=off,hickory_proto=off,tokio_tungstenite=off,tungstenite=off,mio=off,want=off");
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
    /// Pull a sample feed from a source network (handled asynchronously in `main`).
    FetchNetwork(NetworkChoice),
    /// The sentence did not match any known command.
    Unrecognised(String),
    /// The user asked to quit the harness.
    Quit,
}

/// Analyse a typed sentence and route it to the correct subsystem.
/// `t` composes the greeting; `tn`/`tb`/`tm`/`th` pull a sample feed from nostr,
/// Bluesky, Mastodon, and Hashiverse respectively, each from a baked-in default
/// identifier.
fn analyse_and_dispatch_sentence(sentence: &str, recipient_name: &str) -> SentenceDispatchOutcome {
    let trimmed_sentence: &str = sentence.trim();
    match trimmed_sentence {
        "q" | "quit" | "exit" => SentenceDispatchOutcome::Quit,
        "t" => SentenceDispatchOutcome::Handled(compose_greeting_message(recipient_name)),
        "tn" => SentenceDispatchOutcome::FetchNetwork(NetworkChoice::Nostr),
        "tb" => SentenceDispatchOutcome::FetchNetwork(NetworkChoice::Bluesky),
        "tm" => SentenceDispatchOutcome::FetchNetwork(NetworkChoice::Mastodon),
        "th" => SentenceDispatchOutcome::FetchNetwork(NetworkChoice::Hashiverse),
        _ => SentenceDispatchOutcome::Unrecognised(trimmed_sentence.to_string()),
    }
}

/// Well-known identifiers used for the harness sample feeds.
const NOSTR_AUTHOR_NPUB: &str = "npub1wmr34t36fy03m8hvgl96zl3znndyzyaqhwmwdtshwmtkg03fetaqhjg240";
const BLUESKY_ACTOR: &str = "jay.bsky.team";
const MASTODON_ACCOUNT: &str = "@Gargron@mastodon.social";
const HASHIVERSE_USER_ID: &str = "ddd86177f252f0d33f32aa3e59fb6b554969faad48af443347c5b72ac2e186f0";

/// Pull a small sample feed from the chosen network and render it as text.
async fn fetch_network_sample(network_choice: NetworkChoice) -> String {
    let fetch_result: anyhow::Result<Vec<AggregatedPost>> = match network_choice {
        NetworkChoice::Nostr => nostr::fetch_recent_posts(NOSTR_AUTHOR_NPUB, nostr::DEFAULT_RELAYS, 5, Duration::from_secs(20)).await,
        NetworkChoice::Bluesky => bluesky::fetch_recent_posts(&default_http_transport(), BLUESKY_ACTOR, 5).await,
        NetworkChoice::Mastodon => mastodon::fetch_recent_posts(&default_http_transport(), MASTODON_ACCOUNT, 5).await,
        NetworkChoice::Hashiverse => fetch_hashiverse_sample().await,
    };

    match fetch_result {
        Ok(posts) => render_posts(network_choice, &posts),
        Err(fetch_error) => format!("{network_choice:?} fetch failed: {fetch_error:#}"),
    }
}

/// Build a Hashiverse client with native defaults and pull the timeline of
/// `HASHIVERSE_USER_ID` (overridable via `MUXSOCIAL_HASHIVERSE_TEST_USER_ID`).
async fn fetch_hashiverse_sample() -> anyhow::Result<Vec<AggregatedPost>> {
    let user_id_hex = std::env::var("MUXSOCIAL_HASHIVERSE_TEST_USER_ID").unwrap_or_else(|_| HASHIVERSE_USER_ID.to_string());

    let hashiverse_client = HashiverseBuilder::new().data_dir(std::env::temp_dir().join("muxsocial-hashiverse-harness")).build_with_keyphrase("muxsocial-harness").await?;

    hashiverse::fetch_recent_posts(&hashiverse_client, &user_id_hex).await
}

fn render_posts(network_choice: NetworkChoice, posts: &[AggregatedPost]) -> String {
    let mut rendered = format!("{network_choice:?}: {} post(s)\n", posts.len());
    for post in posts {
        let author = post.author_display_name.as_deref().unwrap_or(&post.author_identifier);
        let preview: String = post.content_text.chars().take(160).collect();
        rendered.push_str(&format!("  [{}] {author}: {preview}\n", post.created_at_millis));
    }
    rendered
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let test_harness_arguments: TestHarnessArguments = TestHarnessArguments::parse();
    configure_logging_listener(&test_harness_arguments.log_level);

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
            SentenceDispatchOutcome::FetchNetwork(network_choice) => {
                println!("{}", fetch_network_sample(network_choice).await);
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
            assert_eq!(dispatch_outcome, SentenceDispatchOutcome::FetchNetwork(expected_network), "expected {command:?} to route to {expected_network:?}");
        }
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
