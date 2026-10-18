//! The original's definitions: `armcom_defs.py` and
//! `armcom_vehicle_defs.py`, converted once to JSON by
//! `tools/defs2json.py`, and the campaign XML files, read as they are by
//! `LoadCampaignInfo`'s rules. All of it is built into the binary.

use crate::color::Color;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize, Debug, Clone)]
pub struct CrewSkill {
    pub name: String,
    pub desc: String,
    /// Positions that can take the skill; empty means any.
    pub restrictions: Vec<String>,
    /// Activation chance per level; `[100]` if always active.
    pub levels: Vec<i32>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct CrewOrder {
    pub name: String,
    pub desc: String,
    pub spot: bool,
    /// Positions that can use the order; empty means any.
    pub position_list: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Damage {
    pub name: String,
    /// Order required to repair, '' if none.
    pub order: String,
    pub repair_score: i32,
    pub break_score: i32,
    pub break_result: String,
    pub auto_repair: bool,
}

/// One frame of a hit animation: glyph, colour, milliseconds.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct AnimFrame(pub u8, pub Color, pub u64);

/// Rank: short name, full name, experience needed for promotion.
#[derive(Deserialize, Debug, Clone)]
pub struct Rank(pub String, pub String, pub i32);

/// Decoration: name, citation, victory points needed.
#[derive(Deserialize, Debug, Clone)]
pub struct Decoration(pub String, pub String, pub i32);

/// Map hex: hx, hy, range band (-1 for the player's hex), sector.
#[derive(Deserialize, Debug, Clone, Copy)]
pub struct HexDef(pub i32, pub i32, pub i32, pub i32);

#[derive(Deserialize, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct Defs {
    pub ammo_types: Vec<String>,
    pub ap_hit_animation: Vec<AnimFrame>,
    pub he_hit_animation: Vec<AnimFrame>,
    pub can_hometowns: Vec<String>,
    pub usa_hometowns: Vec<String>,
    pub check_area: Vec<String>,
    pub move_area: Vec<String>,
    pub credits_text: Vec<String>,
    pub crew_orders: Vec<CrewOrder>,
    pub crew_talk_armour_saved: Vec<String>,
    pub crew_talk_arty_strike: Vec<String>,
    pub crew_talk_heavy_res: Vec<String>,
    pub crew_talk_no_arty_strike: Vec<String>,
    pub crew_talk_no_res: Vec<String>,
    pub crew_talk_shot_missed: Vec<String>,
    pub crew_talk_thrown_track: Vec<String>,
    pub c_stats: Vec<String>,
    pub damage_types: Vec<Damage>,
    pub ending_damages: Vec<String>,
    pub fire_gun_info: Vec<String>,
    pub fire_mgs_info: Vec<String>,
    pub first_names: Vec<String>,
    pub last_names: Vec<String>,
    /// (title, text) pairs.
    pub help_text: Vec<(String, String)>,
    pub hexes: Vec<HexDef>,
    pub level_cap: i32,
    pub menu_bar1: String,
    pub menu_bar2: String,
    pub orders_phase_info: Vec<String>,
    pub order_info: Vec<String>,
    pub pivot_info: Vec<String>,
    pub rotate_info: Vec<String>,
    pub skills: Vec<CrewSkill>,
    pub spot_sector_info: Vec<String>,
    pub tank_names: Vec<String>,
    pub tutorial_text: std::collections::HashMap<String, String>,
    pub unit_info: std::collections::HashMap<String, String>,
    pub usa_ranks: Vec<Rank>,
    pub usa_decorations: Vec<Decoration>,
    pub ukc_ranks: Vec<Rank>,
    pub ukc_decorations: Vec<Decoration>,
}

impl Defs {
    /// `GetDamageType`
    #[allow(dead_code)]
    pub fn damage_type(&self, name: &str) -> Option<&Damage> {
        self.damage_types.iter().find(|d| d.name == name)
    }
}

/// A vehicle stat value as the original writes them: a string, a number
/// or a list of numbers (`rarity`).
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Stat {
    Int(i32),
    Str(String),
    List(Vec<i32>),
}

/// One entry of `VEHICLE_TYPES`: its id and its stats in the original's
/// order. Some stats are flags that only matter by being present
/// (`'HVSS' in stats`), so this stays a keyed list rather than a struct.
#[derive(Deserialize, Debug, Clone)]
pub struct VehicleType {
    pub id: String,
    stats: Vec<(String, Stat)>,
}

#[allow(dead_code)]
impl VehicleType {
    pub fn get(&self, key: &str) -> Option<&Stat> {
        self.stats.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }

    pub fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Stat::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn int(&self, key: &str) -> Option<i32> {
        match self.get(key)? {
            Stat::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn list(&self, key: &str) -> Option<&[i32]> {
        match self.get(key)? {
            Stat::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn stats(&self) -> impl Iterator<Item = (&str, &Stat)> {
        self.stats.iter().map(|(k, v)| (k.as_str(), v))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
struct Vehicles {
    vehicle_types: Vec<VehicleType>,
}

pub fn defs() -> &'static Defs {
    static DEFS: OnceLock<Defs> = OnceLock::new();
    DEFS.get_or_init(|| serde_json::from_str(include_str!("../data/defs.json")).expect("data/defs.json"))
}

pub fn vehicle_types() -> &'static [VehicleType] {
    static VEHICLES: OnceLock<Vec<VehicleType>> = OnceLock::new();
    VEHICLES.get_or_init(|| {
        serde_json::from_str::<Vehicles>(include_str!("../data/vehicles.json"))
            .expect("data/vehicles.json")
            .vehicle_types
    })
}

#[allow(dead_code)]
pub fn vehicle_type(id: &str) -> Option<&'static VehicleType> {
    vehicle_types().iter().find(|v| v.id == id)
}

// ---------------------------------------------------------------------
// Campaign files

/// The campaign XML files in the original `data/`, built into the binary.
pub const CAMPAIGN_FILES: &[(&str, &str)] = &[
    ("canadas_best.xml", include_str!("../data/canadas_best.xml")),
    ("pattons_best.xml", include_str!("../data/pattons_best.xml")),
];

/// Unit classes in the order `LoadCampaignInfo` reads them.
pub const CLASSES: [&str; 8] = ["TANK", "SPG", "AT_GUN", "LW", "MG", "TRUCK", "APC", "AC"];

#[derive(Debug, Clone)]
pub struct ActivationModifier {
    pub year: i32,
    pub month: i32,
    pub date: i32,
    pub class_name: String,
    pub r#mod: i32,
}

/// A calendar day. Optional keys are `None` when the file leaves them out.
#[derive(Debug, Clone, Default)]
pub struct Day {
    pub month: i32,
    pub date: i32,
    pub year: i32,
    pub comment: String,
    pub resistance_level: Option<String>,
    pub mission: Option<String>,
    pub description: Option<String>,
    pub terrain: Option<String>,
    pub map_x: Option<String>,
    pub map_y: Option<String>,
}

/// What `LoadCampaignInfo` and the campaign chooser read from a file.
#[derive(Debug, Clone)]
pub struct CampaignDef {
    pub name: String,
    pub description: String,
    pub player_nation: String,
    pub enemy_nation: String,
    pub map_file: Option<String>,
    pub player_veh_list: Vec<String>,
    /// Per mission (advance, battle, counterattack): (class, chance).
    pub mission_activations: Vec<Vec<(String, i32)>>,
    pub activation_modifiers: Vec<ActivationModifier>,
    /// Per class in `CLASSES` order: (unit type, chance out of 1000).
    pub class_activations: Vec<(String, Vec<(String, i32)>)>,
    pub days: Vec<Day>,
}

fn text<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Option<&'a str> {
    node.children().find(|n| n.has_tag_name(tag)).map(|n| n.text().unwrap_or(""))
}

fn child<'a>(node: roxmltree::Node<'a, 'a>, tag: &str) -> Result<roxmltree::Node<'a, 'a>, String> {
    node.children().find(|n| n.has_tag_name(tag)).ok_or_else(|| format!("missing <{tag}>"))
}

fn int(node: roxmltree::Node, tag: &str) -> Result<i32, String> {
    let t = text(node, tag).ok_or_else(|| format!("missing <{tag}>"))?;
    t.trim().parse().map_err(|_| format!("<{tag}> is not a number: {t:?}"))
}

pub fn load_campaign(xml: &str) -> Result<CampaignDef, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();
    let req = |tag| text(root, tag).map(str::to_string).ok_or_else(|| format!("missing <{tag}>"));

    let mut mission_activations = Vec::new();
    let table = child(root, "activation_table")?;
    for tag in ["advance", "battle", "counterattack"] {
        let mission = child(table, tag)?;
        let row = CLASSES.iter().map(|c| Ok((c.to_string(), int(mission, c)?))).collect::<Result<_, String>>()?;
        mission_activations.push(row);
    }

    let mut activation_modifiers = Vec::new();
    if let Ok(mods) = child(root, "activation_modifiers") {
        for m in mods.children().filter(|n| n.has_tag_name("modifier")) {
            activation_modifiers.push(ActivationModifier {
                year: int(m, "year")?,
                month: int(m, "month")?,
                date: int(m, "date")?,
                class_name: text(m, "class_name").unwrap_or("").to_string(),
                r#mod: int(m, "mod")?,
            });
        }
    }

    // each entry is "unit type; chance;" read with csv.reader(delimiter=';',
    // skipinitialspace=True)
    let units = child(root, "unit_class_activations")?;
    let mut class_activations = Vec::new();
    for class in CLASSES {
        let mut list = Vec::new();
        for n in units.children().filter(|n| n.has_tag_name(class)) {
            let line = n.text().unwrap_or("");
            let mut fields = line.split(';').map(str::trim_start);
            let name = fields.next().unwrap_or("").to_string();
            let chance = fields.next().unwrap_or("").trim().parse().map_err(|_| format!("bad <{class}>: {line:?}"))?;
            list.push((name, chance));
        }
        class_activations.push((class.to_string(), list));
    }

    let mut days = Vec::new();
    for d in child(root, "calendar")?.children().filter(|n| n.has_tag_name("day")) {
        let opt = |tag| text(d, tag).map(str::to_string);
        days.push(Day {
            month: int(d, "month")?,
            date: int(d, "date")?,
            year: int(d, "year")?,
            comment: text(d, "comment").ok_or("missing <comment>")?.to_string(),
            resistance_level: opt("resistance_level"),
            mission: opt("mission"),
            description: opt("description"),
            terrain: opt("terrain"),
            map_x: opt("map_x"),
            map_y: opt("map_y"),
        });
    }

    Ok(CampaignDef {
        name: req("name")?,
        description: req("description")?,
        player_nation: req("player_nation")?,
        enemy_nation: req("enemy_nation")?,
        map_file: text(root, "campaign_map_file").map(str::to_string),
        player_veh_list: child(root, "player_tanks")?
            .children()
            .filter(|n| n.has_tag_name("player_tank_type"))
            .map(|n| n.text().unwrap_or("").to_string())
            .collect(),
        mission_activations,
        activation_modifiers,
        class_activations,
        days,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn definitions_load() {
        let d = defs();
        assert_eq!(d.hexes.len(), 37);
        assert_eq!(d.skills.len(), 23);
        assert_eq!(d.crew_orders.len(), 32);
        assert_eq!(d.damage_types.len(), 16);
        assert_eq!(d.level_cap, 40);
        assert_eq!(d.he_hit_animation[0].1, Color(191, 191, 0));
        assert!(d.damage_type("Radio Broken").is_some());
    }

    #[test]
    fn vehicles_load() {
        let v = vehicle_types();
        assert_eq!(v.len(), 32);
        let m4 = vehicle_type("M4 Turret A").unwrap();
        assert_eq!(m4.str("main_gun"), Some("75"));
        assert_eq!(m4.int("hull_front_armour"), Some(8));
        assert_eq!(m4.list("rarity"), Some(&[6, 5, 3, 1, 1, 1, 1, 1, 1][..]));
        assert!(!m4.has("smoke_mortar"));
        assert!(vehicle_type("M4 Turret B").unwrap().has("smoke_mortar"));
    }

    #[test]
    fn campaigns_load_and_name_known_vehicles() {
        for (file, xml) in CAMPAIGN_FILES {
            let c = load_campaign(xml).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert!(!c.days.is_empty(), "{file}");
            assert_eq!(c.mission_activations.len(), 3);
            assert_eq!(c.class_activations.len(), CLASSES.len());
            // Patton's Best has listed this one since the first upload, with
            // no entry in VEHICLE_TYPES; GetRF gives it rarity 0, so it is
            // never offered. Kept as the original has it.
            const NO_ENTRY: &str = "M4A3E2(76)W Turret G";
            for id in &c.player_veh_list {
                assert!(vehicle_type(id).is_some() || id == NO_ENTRY, "{file}: unknown player tank {id}");
            }
            if let Some(map) = &c.map_file {
                assert!(crate::xp::FILES.iter().any(|(n, _)| n == map), "{file}: {map}");
            }
        }
        let pb = load_campaign(CAMPAIGN_FILES[1].1).unwrap();
        assert_eq!(pb.name, "Patton's Best");
        assert_eq!(pb.class_activations[0].1[0], ("PzKw IV H".to_string(), 500));
        assert_eq!(pb.days[0].comment, "Cobra Breakout");
        assert_eq!(pb.days[0].map_x.as_deref(), Some("18"));
    }
}
