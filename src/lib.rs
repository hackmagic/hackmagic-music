//! HackMagic Music Player - Core library
//! Shared between CLI (`hm`) and GUI (`hm-gui`) binaries.

#![allow(dead_code)]
#![recursion_limit = "1024"]

pub mod ai;
pub mod audio_common;

use std::sync::OnceLock;

/// Global Tokio runtime shared across the entire application.
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"))
}
pub mod bass;
pub mod charset;
pub mod cli;
pub mod color;
pub mod commands;
pub mod config;
pub mod core;
pub mod error;
pub mod ffmpeg_engine;
#[cfg(target_os = "windows")]
pub mod mci_engine;
pub mod rodio_engine;
pub mod rodio_common;
pub mod symphonia_engine;
pub mod lyric;
pub mod lastfm;
pub mod media;
pub mod online;
pub mod playlist_format;
pub mod cuesheet;
pub mod multi_version;
pub mod osu;
pub mod hotkey;
pub mod rpc;
pub mod util;
pub mod tag;
pub mod vecdb;
pub mod play_stats;
pub mod smtc;
#[cfg(target_os = "windows")]
pub mod tray;
pub mod gui;