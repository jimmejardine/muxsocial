use clap::Parser;
use muxsocial_lib::greeting::compose_greeting_message;

#[derive(Parser, Debug, Clone)]
#[command(name = "test-harness", about = "mux.social long-running integration test harness")]
struct TestHarnessArguments {
    /// Recipient name used for the end-to-end greeting check
    #[arg(long, default_value = "integration-harness")]
    recipient_name: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let test_harness_arguments: TestHarnessArguments = TestHarnessArguments::parse();
    let greeting_message: String = compose_greeting_message(&test_harness_arguments.recipient_name);
    println!("{greeting_message}");
    anyhow::ensure!(greeting_message.contains(&test_harness_arguments.recipient_name), "Greeting did not contain the recipient name");
    Ok(())
}
