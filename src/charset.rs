#![allow(dead_code)]
use encoding_rs::Encoding;

/// Detect text encoding from raw bytes (BOM + heuristic).
pub fn detect_encoding(data: &[u8]) -> &'static str {
    if data.len() < 2 { return "UTF-8"; }
    // BOM detection
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) { return "UTF-8-BOM"; }
    if data.starts_with(&[0xFF, 0xFE]) { return "UTF-16LE"; }
    if data.starts_with(&[0xFE, 0xFF]) { return "UTF-16BE"; }
    // Check if valid UTF-8
    if std::str::from_utf8(data).is_ok() { return "UTF-8"; }
    // Likely GBK or Shift-JIS on Windows
    "GBK"
}

/// Decode bytes to String using detected encoding.
/// Falls back to lossy UTF-8 if all else fails.
pub fn decode_to_string(data: &[u8]) -> String {
    if data.is_empty() { return String::new(); }
    // Try UTF-8 first
    if let Ok(s) = std::str::from_utf8(data) {
        return s.to_string();
    }
    // Try GBK (common for Chinese Windows)
    if let Some(encoding) = Encoding::for_label(b"gbk") {
        let (cow, _) = encoding.decode_without_bom_handling(data);
        if !cow.contains('\u{FFFD}') {
            return cow.into_owned();
        }
    }
    // Try Shift-JIS
    if let Some(encoding) = Encoding::for_label(b"shift_jis") {
        let (cow, _) = encoding.decode_without_bom_handling(data);
        if !cow.contains('\u{FFFD}') {
            return cow.into_owned();
        }
    }
    // Fallback: lossy UTF-8
    String::from_utf8_lossy(data).into_owned()
}

/// Simplified to Traditional Chinese conversion.
/// On Windows uses kernel32 `LCMapStringW` for full Unicode coverage.
/// On other platforms returns original string (install `opencc` crate for full support).
pub fn to_traditional_chinese(s: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn LCMapStringW(locale: u32, dwMapFlags: u32, lpSrcStr: *const u16, cchSrc: i32, lpDestStr: *mut u16, cchDest: i32) -> i32;
        }
        const LCMAP_TRADITIONAL_CHINESE: u32 = 0x0400_0000;
        const LOCALE_CHINESE: u32 = 0x0804;
        let src: Vec<u16> = s.encode_utf16().collect();
        let mut dst = vec![0u16; src.len() * 2 + 16];
        unsafe {
            let len = LCMapStringW(LOCALE_CHINESE, LCMAP_TRADITIONAL_CHINESE, src.as_ptr(), src.len() as i32, dst.as_mut_ptr(), dst.len() as i32);
            if len > 0 {
                dst.truncate(len as usize);
                return String::from_utf16_lossy(&dst);
            }
        }
    }
    s.to_string()
}

/// Traditional to Simplified Chinese conversion.
pub fn to_simplified_chinese(s: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        extern "system" {
            fn LCMapStringW(locale: u32, dwMapFlags: u32, lpSrcStr: *const u16, cchSrc: i32, lpDestStr: *mut u16, cchDest: i32) -> i32;
        }
        const LCMAP_SIMPLIFIED_CHINESE: u32 = 0x0200_0000;
        const LOCALE_CHINESE: u32 = 0x0804;
        let src: Vec<u16> = s.encode_utf16().collect();
        let mut dst = vec![0u16; src.len() * 2 + 16];
        unsafe {
            let len = LCMapStringW(LOCALE_CHINESE, LCMAP_SIMPLIFIED_CHINESE, src.as_ptr(), src.len() as i32, dst.as_mut_ptr(), dst.len() as i32);
            if len > 0 {
                dst.truncate(len as usize);
                return String::from_utf16_lossy(&dst);
            }
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_utf8() {
        assert_eq!(detect_encoding(b"hello"), "UTF-8");
        assert_eq!(detect_encoding(b"\xef\xbb\xbfhello"), "UTF-8-BOM");
    }

    #[test]
    fn test_detect_utf16() {
        assert_eq!(detect_encoding(b"\xff\xfeh\x00"), "UTF-16LE");
        assert_eq!(detect_encoding(b"\xfe\xff\x00h"), "UTF-16BE");
    }

    #[test]
    fn test_decode_utf8() {
        assert_eq!(decode_to_string(b"hello"), "hello");
    }
}
