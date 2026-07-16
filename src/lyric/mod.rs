pub mod parser;
pub mod editor;

pub use parser::{
    parse_lrc, parse_lrc_str, parse_ksc, parse_vtt, parse_lrc_with_mode,
    load_lyric_file,
    lyrics_to_traditional, lyrics_to_simplified,
    LyricLine, KscWord, Lyrics, TranslateMode,
};
pub use editor::{
    format_time_ms, parse_time_ms, to_lrc_string,
    adjust_timestamp, shift_all_timestamps,
};
