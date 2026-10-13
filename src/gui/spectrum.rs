//! Spectrum strip rendering for the now-playing view.
//!
//! Kept out of `mod.rs` (7.7k lines): the strip is a pure function of its
//! arguments and touches no `MusicPlayer` state.

use super::theme::UiColors;
use gpui::{div, px};
use gpui_component::{h_flex, v_flex};
use gpui::prelude::*;

/// Render the now-playing spectrum strip (bars + peak markers).
pub(crate) fn render_strip(
    raw_spec: &[f32],
    raw_peaks: &[f32],
    cols: usize,
    c: &UiColors,
) -> impl IntoElement {
    let bar_spacing = 2.0;
    let bar_count = cols.min(64);
    let bar_w = 4.0;
    let strip_h = 36.0;
    let max_h = strip_h - 4.0;

    // sample spectrum data down to bar_count
    let bars: Vec<f32> = if raw_spec.len() >= bar_count {
        let step = raw_spec.len() / bar_count;
        (0..bar_count)
            .map(|i| {
                let idx = (i * step).min(raw_spec.len() - 1);
                (raw_spec[idx] * max_h).min(max_h).max(1.0)
            })
            .collect()
    } else {
        raw_spec
            .iter()
            .map(|v| (v * max_h).min(max_h).max(1.0))
            .chain(std::iter::repeat_n(
                1.0,
                bar_count.saturating_sub(raw_spec.len()),
            ))
            .take(bar_count)
            .collect()
    };

    let peak_vals: Vec<f32> = if raw_peaks.len() >= bar_count {
        let step = raw_peaks.len() / bar_count;
        (0..bar_count)
            .map(|i| {
                let idx = (i * step).min(raw_peaks.len() - 1);
                (raw_peaks[idx] * max_h).min(max_h)
            })
            .collect()
    } else {
        vec![0.0; bar_count]
    };

    h_flex()
        .w_full()
        .h(px(strip_h))
        .px_4()
        .items_end()
        .gap(px(bar_spacing))
        .children(bars.iter().enumerate().map(|(i, &bar_h)| {
            let peak_h = peak_vals[i];
            let show_peak = peak_h > bar_h + 2.0;
            v_flex()
                .items_center()
                .h(px(max_h))
                .w(px(bar_w))
                .child(if show_peak {
                    div().w(px(3.0)).h(px(2.0)).bg(c.accent).into_any_element()
                } else {
                    div().w(px(3.0)).h(px(0.0)).into_any_element()
                })
                .child(div().w_full().h(px(bar_h)).bg(c.accent).rounded(px(1.0)))
            }))
    }
