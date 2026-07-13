use crate::cli::{EqualizerArgs, EqualizerAction, ReverbArgs, ReverbAction, ABRepeatArgs, ABRepeatAction};
use crate::commands::get_player;
use crate::error::Result;

const EQ_FREQ: [&str; 10] = ["31Hz", "62Hz", "125Hz", "250Hz", "500Hz", "1kHz", "2kHz", "4kHz", "8kHz", "16kHz"];

fn print_eq_bar(gain: i32) {
    let center: usize = 15;
    let offset = gain.clamp(-15, 15);
    let pos = if offset >= 0 { center + offset as usize } else { center - (-offset) as usize };
    let mut bar = ['░'; 31];
    if offset >= 0 {
        bar[center..=pos].fill('█');
    } else {
        bar[pos..=center].fill('█');
    }
    let bar_str: String = bar.iter().collect();
    if gain >= 0 {
        print!(" +{gain:>2} dB {bar_str}");
    } else {
        print!(" {gain:>2} dB {bar_str}");
    }
}

pub fn cmd_eq(args: &EqualizerArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        EqualizerAction::Get(v) => {
            if let Some(band) = v.band {
                let gain = player.eq_get_band(band);
                println!("Band {} ({}): {} dB", band, EQ_FREQ[band], gain);
            } else {
                let enabled = player.eq_is_enabled();
                let status = if enabled { "ENABLED" } else { "DISABLED" };
                println!("Equalizer: [{status}]");
                println!("{}", "─".repeat(48));
                let gains = player.eq_get();
                for (i, g) in gains.iter().enumerate() {
                    print!("  Band {:2} ({:>5}): ", i, EQ_FREQ[i]);
                    print_eq_bar(*g);
                    println!();
                }
                println!("{}", "─".repeat(48));
                println!("  {:>21}  ↑         0 dB         ↓", "");
            }
        }
        EqualizerAction::Set(v) => {
            player.eq_set(v.band, v.gain)?;
            println!("Equalizer band {} ({}): {} dB", v.band, EQ_FREQ[v.band], v.gain);
        }
        EqualizerAction::Preset(v) => {
            player.eq_set_preset(&v.style)?;
            println!("Equalizer preset: {}", v.style);
            // Show the preset gains
            let gains = player.eq_get();
            for (i, g) in gains.iter().enumerate() {
                println!("  Band {:2} ({}): {:>3} dB", i, EQ_FREQ[i], g);
            }
        }
        EqualizerAction::Enable => player.eq_enable(true),
        EqualizerAction::Disable => player.eq_enable(false),
        EqualizerAction::Reset => player.eq_reset()?,
    }
    Ok(())
}

pub fn cmd_reverb(args: &ReverbArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        ReverbAction::Get => {
            let enabled = player.reverb_is_enabled();
            let status = if enabled { "ENABLED" } else { "DISABLED" };
            let (mix, time) = player.reverb_get();
            let mix_bar_cnt = (mix as f32 / 100.0 * 20.0) as usize;
            let mix_bar: String = (0..20).map(|i| if i < mix_bar_cnt { '█' } else { '░' }).collect();
            println!("Reverb: [{status}]");
            println!("{}", "─".repeat(40));
            println!("  Mix : {mix:>3}%  [{mix_bar}]");
            println!("  Time: {time:>4} ms");
            println!("{}", "─".repeat(40));
        }
        ReverbAction::Mix(v) => {
            let (_, time) = player.reverb_get();
            player.reverb_set(v.mix, time)?;
            println!("Reverb mix set to {}%", v.mix);
        }
        ReverbAction::Time(v) => {
            let (mix, _) = player.reverb_get();
            player.reverb_set(mix, v.time)?;
            println!("Reverb time set to {} ms", v.time);
        }
        ReverbAction::Enable => {
            let (mix, time) = player.reverb_get();
            // Use defaults if never configured
            let mix = if mix == 0 { 50 } else { mix };
            let time = if time <= 1 { 100 } else { time };
            player.reverb_set(mix, time)?;
            println!("Reverb enabled (mix={mix}%, time={time}ms)");
        }
        ReverbAction::Disable => {
            player.reverb_clear()?;
            println!("Reverb disabled");
        }
    }
    Ok(())
}

pub fn cmd_ab(args: &ABRepeatArgs) -> Result<()> {
    let player = get_player();
    match &args.action {
        ABRepeatAction::SetA => player.ab_set_a(),
        ABRepeatAction::SetB => player.ab_set_b(),
        ABRepeatAction::Reset => { player.ab_reset(); Ok(()) }
        ABRepeatAction::Continue => player.ab_continue(),
        ABRepeatAction::Status => {
            let status = player.ab_status();
            match status.mode {
                crate::core::player::ABRepeatMode::None => println!("AB repeat: off"),
                crate::core::player::ABRepeatMode::ASelected => {
                    println!("A: {:.1}s (awaiting B)", status.a.as_secs_f64());
                }
                crate::core::player::ABRepeatMode::ABRepeat => {
                    println!("A: {:.1}s \u{2192} B: {:.1}s", status.a.as_secs_f64(), status.b.as_secs_f64());
                }
            }
            Ok(())
        }
    }
}
