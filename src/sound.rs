//! Sound (`InitMixer` and `PlaySound` in armcom.py): the original's
//! effects, `sounds/*.wav` (GPL with the rest), built into the binary.
//! The original mixed them on 16 SDL channels; here one audio thread owns
//! the output device and mixes as many as are playing. Without an audio
//! device the game plays on silently. Tests never open the device.

use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};

/// `SOUND_LIST`, with the files. (`75mm_gun_firing.wav` is in the
/// original's folder but not its list, so it is left out.)
const SOUNDS: &[(&str, &[u8])] = &[
    ("20_mm_gun", include_bytes!("../data/sounds/20_mm_gun.wav")),
    ("75_mm_gun", include_bytes!("../data/sounds/75_mm_gun.wav")),
    ("76_mm_gun", include_bytes!("../data/sounds/76_mm_gun.wav")),
    ("88_mm_gun", include_bytes!("../data/sounds/88_mm_gun.wav")),
    ("main_gun_misfire", include_bytes!("../data/sounds/main_gun_misfire.wav")),
    ("menu_select", include_bytes!("../data/sounds/menu_select.wav")),
    ("aa_mg_firing", include_bytes!("../data/sounds/aa_mg_firing.wav")),
    ("armour_save", include_bytes!("../data/sounds/armour_save.wav")),
    ("arty_firing", include_bytes!("../data/sounds/arty_firing.wav")),
    ("bow_mg_firing", include_bytes!("../data/sounds/bow_mg_firing.wav")),
    ("coax_mg_firing", include_bytes!("../data/sounds/coax_mg_firing.wav")),
    ("dice_roll", include_bytes!("../data/sounds/dice_roll.wav")),
    ("engine_noise", include_bytes!("../data/sounds/engine_noise.wav")),
    ("german_rifle_fire", include_bytes!("../data/sounds/german_rifle_fire.wav")),
    ("german_mg_fire", include_bytes!("../data/sounds/german_mg_fire.wav")),
    ("infantry_moving", include_bytes!("../data/sounds/infantry_moving.wav")),
    ("panzerfaust_firing", include_bytes!("../data/sounds/panzerfaust_firing.wav")),
    ("radio", include_bytes!("../data/sounds/radio.wav")),
    ("screenshot", include_bytes!("../data/sounds/screenshot.wav")),
    ("shell_move", include_bytes!("../data/sounds/shell_move.wav")),
    ("sherman_movement", include_bytes!("../data/sounds/sherman_movement.wav")),
    ("smoke_hit", include_bytes!("../data/sounds/smoke_hit.wav")),
    ("tank_knocked_out", include_bytes!("../data/sounds/tank_knocked_out.wav")),
    ("hatch_open", include_bytes!("../data/sounds/hatch_open.wav")),
    ("hatch_close", include_bytes!("../data/sounds/hatch_close.wav")),
    ("he_hit", include_bytes!("../data/sounds/he_hit.wav")),
    ("ap_hit", include_bytes!("../data/sounds/ap_hit.wav")),
    ("main_gun_miss", include_bytes!("../data/sounds/main_gun_miss.wav")),
    ("new_skill", include_bytes!("../data/sounds/new_skill.wav")),
];

/// The audio thread's queue; `None` once it is known there is no audio.
static QUEUE: OnceLock<Option<Sender<&'static [u8]>>> = OnceLock::new();

fn queue() -> Option<&'static Sender<&'static [u8]>> {
    QUEUE
        .get_or_init(|| {
            if cfg!(test) {
                return None;
            }
            let (tx, rx) = mpsc::channel::<&'static [u8]>();
            let spawned = std::thread::Builder::new().name("audio".into()).spawn(move || {
                let mut sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("no audio device, playing without sound: {e}");
                        return;
                    }
                };
                // rodio would print to the terminal, over the game
                sink.log_on_drop(false);
                log::info!("sound: audio device open");
                for data in rx {
                    match rodio::Decoder::new(std::io::Cursor::new(data)) {
                        Ok(source) => sink.mixer().add(source),
                        Err(e) => log::error!("sound: cannot decode: {e}"),
                    }
                }
            });
            spawned.ok().map(|_| tx)
        })
        .as_ref()
}

/// `PlaySound`: start a sound, alongside any already playing.
pub fn play(name: &str) {
    let Some((_, data)) = SOUNDS.iter().find(|(n, _)| *n == name) else {
        log::warn!("sound: unknown {name}");
        return;
    };
    log::trace!("sound: {name}");
    if let Some(tx) = queue() {
        let _ = tx.send(data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_sound_decodes() {
        for (name, data) in SOUNDS {
            assert!(rodio::Decoder::new(std::io::Cursor::new(*data)).is_ok(), "{name}");
        }
    }

    #[test]
    fn every_sound_the_game_plays_is_there() {
        let names: Vec<&str> = SOUNDS.iter().map(|(n, _)| *n).collect();
        for file in ["calendar", "campaign", "combat", "crew", "day", "encounter", "enemy", "screens", "tank", "game"] {
            let text = std::fs::read_to_string(format!("src/{file}.rs")).unwrap();
            for part in text.split("play_sound(\"").skip(1) {
                let name = &part[..part.find('"').unwrap()];
                assert!(names.contains(&name), "{file}.rs plays {name}");
            }
        }
        for gun in ["20L", "50L", "75", "75L", "75LL", "76L", "76LL", "88L", "88LL"] {
            let s = crate::enemy::firing_sound(gun).unwrap();
            assert!(names.contains(&s), "{gun}: {s}");
        }
    }
}
