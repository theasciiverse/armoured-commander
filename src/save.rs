//! The saved game (`SaveGame`, `LoadGame` and `SavedGameInfo` in
//! armcom.py). The original shelved the campaign, tank and battle objects
//! beside the game; this is the same three as JSON in the player's data
//! folder, with the summary the main menu shows. One campaign at a time,
//! as in the original.

use crate::bones;
use crate::model::{Battle, Campaign, PlayerTank};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Bumped whenever the saved state changes shape; a save of another
/// format is reported on the main menu rather than loaded.
pub const FORMAT: u32 = 1;

/// `SavedGameInfo`: what the main menu shows of the saved game.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Info {
    pub format: u32,
    pub game_version: String,
    pub campaign_name: String,
    pub commander_name: String,
    pub tank_name: String,
    pub current_date: String,
}

#[derive(Serialize)]
struct SaveRef<'a> {
    info: &'a Info,
    campaign: &'a Campaign,
    tank: &'a PlayerTank,
    battle: &'a Option<Battle>,
}

#[derive(Deserialize)]
struct SaveFile {
    info: Info,
    campaign: Campaign,
    tank: PlayerTank,
    battle: Option<Battle>,
}

/// Just the summary, for reading it without the rest.
#[derive(Deserialize)]
struct InfoOnly {
    info: Info,
}

fn path() -> Option<PathBuf> {
    bones::data_dir().map(|d| d.join("savegame.json"))
}

pub fn to_string(info: &Info, campaign: &Campaign, tank: &PlayerTank, battle: &Option<Battle>) -> Result<String, String> {
    serde_json::to_string(&SaveRef { info, campaign, tank, battle }).map_err(|e| e.to_string())
}

pub fn from_str(text: &str) -> Result<(Campaign, PlayerTank, Option<Battle>), String> {
    let f: SaveFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if f.info.format != FORMAT {
        return Err(format!("saved by another version (format {})", f.info.format));
    }
    Ok((f.campaign, f.tank, f.battle))
}

/// Write the game, through a temporary file so that a save cut short
/// leaves the last one whole.
pub fn write(info: &Info, campaign: &Campaign, tank: &PlayerTank, battle: &Option<Battle>) {
    let Some(p) = path() else { return };
    let result = to_string(info, campaign, tank, battle).and_then(|text| {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
    });
    match result {
        Ok(()) => log::debug!("saved to {}", p.display()),
        Err(e) => log::error!("could not save the game: {e}"),
    }
}

/// The saved game's summary: `None` if there is no saved game, an error
/// if it cannot be read or is of another format.
pub fn peek() -> Option<Result<Info, String>> {
    let text = std::fs::read_to_string(path()?).ok()?;
    Some(match serde_json::from_str::<InfoOnly>(&text) {
        Ok(f) if f.info.format == FORMAT => Ok(f.info),
        Ok(f) => Err(format!("saved by another version (format {})", f.info.format)),
        Err(e) => Err(e.to_string()),
    })
}

pub fn load() -> Result<(Campaign, PlayerTank, Option<Battle>), String> {
    let p = path().ok_or("no data folder")?;
    let text = std::fs::read_to_string(p).map_err(|e| e.to_string())?;
    from_str(&text)
}

/// `os.remove('savegame')`, when the campaign is over.
pub fn delete() {
    if let Some(p) = path() {
        let _ = std::fs::remove_file(p);
    }
}
