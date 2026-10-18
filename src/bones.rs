//! The bones file (`Bones` in armcom.py): high scores, the graveyard of
//! fallen crew, and which tutorial messages have been shown, kept between
//! campaigns. The original pickled it with `shelve` beside the game; this
//! is JSON in the player's data folder.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Score {
    pub tank_name: String,
    pub commander_name: String,
    pub vp: i32,
    pub outcome: String,
    pub unlimited_tank_selection: bool,
    pub casual_commander: bool,
    pub campaign_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bones {
    pub score_list: Vec<Score>,
    /// rank, name, hometown, date, last decoration
    pub graveyard: Vec<[String; 5]>,
    pub tutorial_message_flags: BTreeMap<String, bool>,
}

impl Default for Bones {
    fn default() -> Self {
        let mut flags = BTreeMap::new();
        for key in crate::defs::defs().tutorial_text.keys() {
            flags.insert(key.clone(), false);
        }
        Bones {
            score_list: Vec::new(),
            // tribute to David Bowie, as in the original
            graveyard: vec![["Major", "Jack Celliers", "Brixton", "January 10", ""].map(String::from)],
            tutorial_message_flags: flags,
        }
    }
}

/// The player's data folder for this game.
pub fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("armoured-commander"))
}

fn path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("bones.json"))
}

pub fn load() -> Bones {
    path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(bones: &Bones) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    match serde_json::to_string_pretty(bones) {
        Ok(text) => {
            if let Err(e) = std::fs::write(&p, text) {
                log::error!("could not write the bones file: {e}");
            }
        }
        Err(e) => log::error!("could not write the bones file: {e}"),
    }
}

/// Mark a tutorial message as shown; false if it already was.
pub fn mark_tutorial(key: &str) -> bool {
    let mut b = load();
    if b.tutorial_message_flags.get(key).copied().unwrap_or(false) {
        return false;
    }
    b.tutorial_message_flags.insert(key.to_string(), true);
    save(&b);
    true
}

/// `AddHeadStone`
pub fn add_headstone(entry: [String; 5]) {
    let mut b = load();
    b.graveyard.push(entry);
    save(&b);
}

/// `AddHighScore`: insert, keep sorted by VP, at most `MAX_HS`.
pub fn add_high_score(score: Score) {
    let mut b = load();
    b.score_list.push(score);
    b.score_list.sort_by(|a, b| b.vp.cmp(&a.vp));
    if b.score_list.len() > crate::game::MAX_HS {
        b.score_list.pop();
    }
    save(&b);
}
