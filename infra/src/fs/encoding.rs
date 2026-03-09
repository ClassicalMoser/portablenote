/// Maximum encoded filename length in bytes (before extension).
const MAX_FILENAME_BYTES: usize = 200;

/// Characters that must be percent-encoded for filesystem safety.
fn must_encode(c: char) -> bool {
    matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '%')
        || c.is_control()
}

/// Encode a block name into a filesystem-safe filename (without extension).
///
/// Per the spec: RFC 3986 percent-encoding over filesystem-unsafe characters,
/// control characters, and the percent literal. All other characters — including
/// spaces, unicode, and common punctuation — pass through unmodified.
/// Block names containing `%` are rejected at command time; they never reach here.
///
/// Truncates to `MAX_FILENAME_BYTES` (200 bytes UTF-8). Names exceeding this
/// after encoding are truncated at a character boundary.
pub fn encode_block_filename(name: &str) -> String {
    let mut encoded = String::new();

    for c in name.chars() {
        if must_encode(c) {
            let mut buf = [0u8; 4];
            for byte in c.encode_utf8(&mut buf).bytes() {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        } else {
            encoded.push(c);
        }
    }

    truncate_to_bytes(&encoded, MAX_FILENAME_BYTES)
}

/// Decode a percent-encoded filename back to the original block name.
pub fn decode_block_filename(encoded: &str) -> String {
    let bytes = encoded.as_bytes();
    let mut decoded_bytes = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &String::from_utf8_lossy(&bytes[i + 1..i + 3]),
                16,
            ) {
                decoded_bytes.push(byte);
                i += 3;
                continue;
            }
        }
        decoded_bytes.push(bytes[i]);
        i += 1;
    }

    String::from_utf8_lossy(&decoded_bytes).into_owned()
}

/// Truncate a string to at most `max_bytes` bytes, cutting at a character boundary.
fn truncate_to_bytes(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_name_passes_through() {
        assert_eq!(encode_block_filename("Getting Started"), "Getting Started");
    }

    #[test]
    fn spaces_are_preserved() {
        assert_eq!(encode_block_filename("My Block Name"), "My Block Name");
    }

    #[test]
    fn colon_is_encoded() {
        assert_eq!(encode_block_filename("Notes: Part 1"), "Notes%3A Part 1");
    }

    #[test]
    fn slash_is_encoded() {
        assert_eq!(encode_block_filename("A/B"), "A%2FB");
    }

    #[test]
    fn percent_is_encoded() {
        assert_eq!(encode_block_filename("100%"), "100%25");
    }

    #[test]
    fn multiple_unsafe_chars() {
        let encoded = encode_block_filename("a:b*c?d");
        assert_eq!(encoded, "a%3Ab%2Ac%3Fd");
    }

    #[test]
    fn unicode_passes_through() {
        assert_eq!(encode_block_filename("Café Culture"), "Café Culture");
    }

    #[test]
    fn round_trip_simple() {
        let name = "Getting Started";
        assert_eq!(decode_block_filename(&encode_block_filename(name)), name);
    }

    #[test]
    fn round_trip_with_encoded_chars() {
        let name = "Notes: Part 1";
        assert_eq!(decode_block_filename(&encode_block_filename(name)), name);
    }

    #[test]
    fn round_trip_unicode() {
        let name = "Café Culture";
        assert_eq!(decode_block_filename(&encode_block_filename(name)), name);
    }

    #[test]
    fn truncation_respects_char_boundary() {
        let long_name = "A".repeat(250);
        let encoded = encode_block_filename(&long_name);
        assert!(encoded.len() <= MAX_FILENAME_BYTES);
    }

    #[test]
    fn short_names_are_not_truncated() {
        let name = "Short";
        assert_eq!(encode_block_filename(name), name);
    }
}
