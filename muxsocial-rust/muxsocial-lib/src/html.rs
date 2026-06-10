//! Small HTML helpers shared by the source clients.
//!
//! Some networks return plain text (nostr, Bluesky's `text` field); to render
//! them as HTML alongside the networks that return HTML natively (Mastodon,
//! Hashiverse), we escape the text and turn newlines into `<br>`.

/// HTML-escape text content (`&`, `<`, `>`). `&` must be replaced first.
pub fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// HTML-escape an attribute value (the text escapes plus `"`).
pub fn escape_attribute(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

/// Wrap a plain-text string into inline HTML: escape it and turn newlines into
/// `<br>`. No linkification or block wrapping.
pub fn plain_text_to_html(text: &str) -> String {
    escape_text(text).replace('\n', "<br>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_text_special_chars() {
        assert_eq!(escape_text("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn escapes_attribute_quotes_as_well() {
        assert_eq!(escape_attribute("a\"b&c"), "a&quot;b&amp;c");
    }

    #[test]
    fn plain_text_escapes_and_converts_newlines() {
        assert_eq!(plain_text_to_html("first <tag>\nsecond & more"), "first &lt;tag&gt;<br>second &amp; more");
    }

    #[test]
    fn plain_text_without_special_chars_passes_through() {
        assert_eq!(plain_text_to_html("just some text"), "just some text");
    }
}
