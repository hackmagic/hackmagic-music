//! ANSI terminal color support for `MusicPlayer2`.
//!
//! Works on Windows Terminal, modern Windows 10+ console (build 10586+),
//! and all Unix terminals. The `enable_ansi_support()` function must be
//! called once at program startup on Windows to enable virtual terminal
//! processing.

/// Reset all attributes
pub const RESET: &str = "\x1b[0m";
/// Bold / bright text
pub const BOLD: &str = "\x1b[1m";
/// Dim / faint text
pub const DIM: &str = "\x1b[2m";

// ── Standard foreground colours ──────────────────────────────────────
pub const BLACK: &str = "\x1b[30m";
pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";
pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const MAGENTA: &str = "\x1b[35m";
pub const CYAN: &str = "\x1b[36m";
pub const WHITE: &str = "\x1b[37m";

// ── Bright foreground colours ────────────────────────────────────────
pub const BRIGHT_RED: &str = "\x1b[91m";
pub const BRIGHT_GREEN: &str = "\x1b[92m";
pub const BRIGHT_YELLOW: &str = "\x1b[93m";
pub const BRIGHT_BLUE: &str = "\x1b[94m";
pub const BRIGHT_MAGENTA: &str = "\x1b[95m";
pub const BRIGHT_CYAN: &str = "\x1b[96m";
pub const BRIGHT_WHITE: &str = "\x1b[97m";

/// Check whether the terminal is likely to support ANSI escape codes.
pub fn supports_color() -> bool {
    #[cfg(windows)]
    {
        // Windows 10 build 10586+ supports ANSI via virtual terminal
        // processing; Windows Terminal supports it natively.
        true
    }
    #[cfg(not(windows))]
    {
        std::env::var("TERM").map_or(true, |t| t != "dumb")
    }
}

/// Wrap `text` in `ansi_code` … `RESET`, or return plain text when the
/// terminal does not support colour.
pub fn colorize(text: &str, ansi_code: &str) -> String {
    if supports_color() {
        format!("{ansi_code}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Enable ANSI escape-sequence processing on the Windows console.
/// No-op on other platforms.  Call once early in `main()`.
#[cfg(windows)]
pub fn enable_ansi_support() {
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5u32;
    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;

    extern "system" {
        fn GetStdHandle(nStdHandle: u32) -> isize;
        fn GetConsoleMode(hConsoleHandle: isize, lpMode: *mut u32) -> i32;
        fn SetConsoleMode(hConsoleHandle: isize, dwMode: u32) -> i32;
    }

    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        // INVALID_HANDLE_VALUE is -1 as isize; avoid dereferencing it.
        // The MSDN docs say GetStdHandle returns INVALID_HANDLE_VALUE on
        // failure, but we also check for NULL just in case.
        let handle_int: isize = handle;
        if handle_int == -1 || handle == 0 {
            return;
        }
        let mut mode: u32 = 0;
        if GetConsoleMode(handle, &raw mut mode) != 0 {
            SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
        }
    }
}

#[cfg(not(windows))]
pub fn enable_ansi_support() {
    // No-op on Unix / other platforms — ANSI is natively supported.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_color() {
        // On Windows, supports_color() always returns true
        // On non-Windows, it checks TERM env var
        #[cfg(windows)]
        assert!(supports_color());
    }

    #[test]
    fn test_colorize_red() {
        let result = colorize("hello", RED);
        assert_eq!(result, format!("{RED}hello{RESET}"));
    }

    #[test]
    fn test_colorize_green() {
        let result = colorize("world", GREEN);
        assert_eq!(result, format!("{GREEN}world{RESET}"));
    }

    #[test]
    fn test_colorize_empty_text() {
        let result = colorize("", RED);
        assert_eq!(result, format!("{RED}{RESET}"));
    }

    #[test]
    fn test_colorize_with_bold() {
        let result = colorize("bold text", BOLD);
        assert_eq!(result, format!("{BOLD}bold text{RESET}"));
    }

    #[test]
    fn test_colorize_bright_colors() {
        let result = colorize("bright", BRIGHT_RED);
        assert_eq!(result, format!("{BRIGHT_RED}bright{RESET}"));
    }

    #[test]
    fn test_colorize_multiple_calls() {
        let r1 = colorize("a", RED);
        let r2 = colorize("b", GREEN);
        let r3 = colorize("c", BLUE);
        assert_eq!(r1, format!("{RED}a{RESET}"));
        assert_eq!(r2, format!("{GREEN}b{RESET}"));
        assert_eq!(r3, format!("{BLUE}c{RESET}"));
    }

    #[test]
    fn test_constants_not_empty() {
        assert!(!RESET.is_empty());
        assert!(!RED.is_empty());
        assert!(!GREEN.is_empty());
        assert!(!BLUE.is_empty());
        assert!(!BOLD.is_empty());
    }

    #[test]
    fn test_colorize_text_with_special_chars() {
        let text = "hello\nworld\t!";
        let result = colorize(text, CYAN);
        assert_eq!(result, format!("{CYAN}hello\nworld\t!{RESET}"));
    }
}
