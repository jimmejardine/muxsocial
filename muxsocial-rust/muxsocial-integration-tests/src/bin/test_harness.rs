use clap::Parser;
use muxsocial_lib::greeting::compose_greeting_message;
use std::io::{self, BufRead, Write};

#[derive(Parser, Debug, Clone)]
#[command(name = "test-harness", about = "mux.social long-running integration test harness")]
struct TestHarnessArguments {
    /// Recipient name used by the `t` greeting command
    #[arg(long, default_value = "integration-harness")]
    recipient_name: String,
}

/// Outcome of analysing one typed sentence in the harness REPL.
#[derive(Debug, PartialEq, Eq)]
enum SentenceDispatchOutcome {
    /// A subsystem handled the sentence and produced output to display.
    Handled(String),
    /// The sentence did not match any known command.
    Unrecognised(String),
    /// The user asked to quit the harness.
    Quit,
}

/// Analyse a typed sentence and route it to the correct subsystem.
/// For now only `t` is wired up (to the greeting test message); everything
/// else is unrecognised. This match is the single extension point for future
/// subsystem dispatch (Hashiverse, nostr, Mastodon, Bluesky).
fn analyse_and_dispatch_sentence(sentence: &str, recipient_name: &str) -> SentenceDispatchOutcome {
    let trimmed_sentence: &str = sentence.trim();
    match trimmed_sentence {
        "q" | "quit" | "exit" => SentenceDispatchOutcome::Quit,
        "t" => SentenceDispatchOutcome::Handled(compose_greeting_message(recipient_name)),
        _ => SentenceDispatchOutcome::Unrecognised(trimmed_sentence.to_string()),
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let test_harness_arguments: TestHarnessArguments = TestHarnessArguments::parse();

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
