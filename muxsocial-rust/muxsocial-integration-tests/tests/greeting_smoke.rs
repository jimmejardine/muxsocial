use muxsocial_lib::greeting::compose_greeting_message;

#[test]
fn compose_greeting_message_round_trips_through_the_integration_crate() {
    let greeting_message: String = compose_greeting_message("integration-smoke");
    assert_eq!(greeting_message, "Hello, integration-smoke! Greetings from muxsocial-lib.");
}
