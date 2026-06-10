/// Composes the hello-world greeting used to verify the lib -> wasm -> web pipeline.
pub fn compose_greeting_message(recipient_name: &str) -> String {
    format!("Hello, {recipient_name}! Greetings again from muxsocial-lib.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_greeting_message_includes_recipient_name() {
        let greeting_message: String = compose_greeting_message("world");
        assert_eq!(greeting_message, "Hello, world! Greetings again from muxsocial-lib.");
    }
}
