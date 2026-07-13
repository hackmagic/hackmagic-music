#![allow(dead_code)]

/// URL-encode a string (percent-encoding)
pub fn url_encode(input: &str) -> String {
    let mut result = String::with_capacity(input.len() * 3);
    for byte in input.bytes() {
        match byte {
            // RFC 3986 unreserved characters
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push('%');
                result.push(hex_nibble(byte >> 4));
                result.push(hex_nibble(byte & 0x0f));
            }
        }
    }
    result
}

/// URL-decode a percent-encoded string
pub fn url_decode(input: &str) -> String {
    let bytes = url_decode_bytes(input);
    String::from_utf8_lossy(&bytes).to_string()
}

fn url_decode_bytes(input: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if hi >= 0 && lo >= 0 {
                result.push((hi as u8) << 4 | lo as u8);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i]);
        i += 1;
    }
    result
}

fn hex_nibble(b: u8) -> char {
    match b & 0x0f {
        0..=9 => (b'0' + (b & 0x0f)) as char,
        _ => (b'A' + (b & 0x0f) - 10) as char,
    }
}

fn hex_val(b: u8) -> i8 {
    match b {
        b'0'..=b'9' => (b - b'0') as i8,
        b'a'..=b'f' => (b - b'a' + 10) as i8,
        b'A'..=b'F' => (b - b'A' + 10) as i8,
        _ => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_unreserved_chars() {
        // RFC 3986 unreserved: A-Z, a-z, 0-9, -, _, ., ~
        assert_eq!(url_encode("abc123"), "abc123");
        assert_eq!(url_encode("ABC"), "ABC");
        assert_eq!(url_encode("-_."), "-_.");
        assert_eq!(url_encode("~"), "~");
    }

    #[test]
    fn test_url_encode_space() {
        assert_eq!(url_encode("hello world"), "hello%20world");
    }

    #[test]
    fn test_url_encode_special_chars() {
        assert_eq!(url_encode("a/b?c#d"), "a%2Fb%3Fc%23d");
        assert_eq!(url_encode("!@$"), "%21%40%24");
        assert_eq!(url_encode("()+"), "%28%29%2B");
    }

    #[test]
    fn test_url_encode_empty_string() {
        assert_eq!(url_encode(""), "");
    }

    #[test]
    fn test_url_encode_utf8() {
        // Chinese characters in UTF-8
        assert_eq!(url_encode("音乐"), "%E9%9F%B3%E4%B9%90");
        // Japanese
        assert_eq!(url_encode("日本語"), "%E6%97%A5%E6%9C%AC%E8%AA%9E");
    }

    #[test]
    fn test_url_encode_mixed() {
        // Mix of unreserved, reserved, and UTF-8
        let result = url_encode("hello 世界!");
        assert!(result.contains("hello"));
        assert!(result.contains("%20"));
        assert!(result.contains("%E4%B8%96%E7%95%8C"));
        assert!(result.contains("%21"));
    }

    #[test]
    fn test_url_encode_percent_sign() {
        assert_eq!(url_encode("100%"), "100%25");
    }

    #[test]
    fn test_url_decode_roundtrip() {
        let inputs = vec![
            "hello world",
            "a/b?c#d",
            "100%",
            "音乐",
            "",
            "!@#$%^&*()",
            "~-_.",
        ];
        for input in inputs {
            let encoded = url_encode(input);
            let decoded = url_decode(&encoded);
            assert_eq!(decoded, input, "roundtrip failed for: {:?}", input);
        }
    }
}
