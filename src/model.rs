//! The original's classes, field for field (armcom.py, "Classes"). Their
//! methods mostly reach for the globals (`campaign`, `tank`, `battle`,
//! the consoles), so they live on `Game` in the module for their part of
//! the game; what is here only touches its own object.
//!
//! Python object references become indices: `battle.target` and
//! `battle.selected_crew` index `battle.enemy_units` and `tank.crew`, a
//! unit's `map_hex` indexes `battle.maphexes`, and map nodes index
//! `day_map.nodes`. The crew's `next`/`prev` ring is the crew list itself.

use crate::defs::{self, CrewOrder, Stats};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

pub const AMMO_TYPES: [&str; 6] = ["HE", "AP", "WP", "HCBI", "HVAP", "APDS"];

/// Main gun shells by type. The original keeps a dict holding only the
/// types the gun fires (`'WP' in tank.general_ammo`), so a missing type
/// is `None`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ammo([Option<i32>; 6]);

impl Ammo {
    fn index(ammo_type: &str) -> usize {
        AMMO_TYPES.iter().position(|t| *t == ammo_type).unwrap_or_else(|| panic!("ammo type {ammo_type}"))
    }

    /// `ammo_type in ammo`
    pub fn has(&self, ammo_type: &str) -> bool {
        AMMO_TYPES.contains(&ammo_type) && self.0[Self::index(ammo_type)].is_some()
    }

    /// `ammo[ammo_type]`; 0 for a type the gun does not fire.
    pub fn get(&self, ammo_type: &str) -> i32 {
        if !AMMO_TYPES.contains(&ammo_type) {
            return 0;
        }
        self.0[Self::index(ammo_type)].unwrap_or(0)
    }

    pub fn set(&mut self, ammo_type: &str, n: i32) {
        self.0[Self::index(ammo_type)] = Some(n);
    }

    pub fn add(&mut self, ammo_type: &str, n: i32) {
        let v = self.get(ammo_type);
        self.set(ammo_type, v + n);
    }

    pub fn clear(&mut self) {
        self.0 = [None; 6];
    }
}

/// `Weather`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub clouds: String,
    pub fog: bool,
    pub precip: String,
    pub ground: String,
    pub rain_time: i32,
    pub snow_time: i32,
    pub dry_time: i32,
}

impl Default for Weather {
    fn default() -> Self {
        Weather {
            clouds: "Clear".into(),
            fog: false,
            precip: "None".into(),
            ground: "Dry".into(),
            rain_time: 0,
            snow_time: 0,
            dry_time: 0,
        }
    }
}

/// `MapNode`: one area of the campaign day map.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MapNode {
    pub x: i32,
    pub y: i32,
    pub edges: BTreeSet<(i32, i32)>,
    pub links: Vec<usize>,
    pub node_type: String,
    pub village_radius: i32,
    pub dirt_road_links: Vec<usize>,
    pub stone_road_links: Vec<usize>,
    pub road_end: bool,
    pub extended: bool,
    pub top_edge: bool,
    pub bottom_edge: bool,
    pub left_edge: bool,
    pub right_edge: bool,
    pub start: bool,
    pub exit: bool,
    pub resistance: Option<String>,
    pub res_known: bool,
    pub friendly_control: bool,
    pub arty_strike: bool,
    pub air_strike: bool,
    pub advancing_fire: bool,
    pub parent: Option<usize>,
    pub g: i32,
    pub h: i32,
    pub f: i32,
    pub quest_type: Option<String>,
    pub quest_time_limit: Option<(i32, i32)>,
    pub quest_vp_bonus: Option<i32>,
}

impl MapNode {
    pub fn new(x: i32, y: i32) -> Self {
        MapNode { x, y, ..Default::default() }
    }

    /// `ClearPathInfo`
    pub fn clear_path_info(&mut self) {
        self.parent = None;
        self.g = 0;
        self.h = 0;
        self.f = 0;
    }
}

/// `CampaignDayMap`
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CampaignDayMap {
    pub seed: u64,
    pub nodes: Vec<MapNode>,
    pub blocked_nodes: BTreeSet<usize>,
    /// Which node each map cell belongs to.
    #[serde(with = "cell_map")]
    pub char_locations: HashMap<(i32, i32), usize>,
    pub player_node: usize,
}

/// `SkillRecord`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillRecord {
    pub name: String,
    pub level: i32,
}

/// `MainGunHit`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MainGunHit {
    pub gun_calibre: String,
    pub ammo_type: String,
    pub critical: bool,
    pub area_fire: bool,
}

/// `MapHex`: x and y are where it is drawn on the map console.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapHex {
    pub hx: i32,
    pub hy: i32,
    pub rng: i32,
    pub sector: i32,
    pub smoke_factors: i32,
    pub x: i32,
    pub y: i32,
}

/// `SmokeFactor`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmokeFactor {
    pub hx: i32,
    pub hy: i32,
    pub num_factors: f64,
}

impl SmokeFactor {
    /// `RotatePosition`: axial to cube, turn a sixth, back to axial.
    pub fn rotate_position(&mut self, clockwise: bool) {
        let (x, z) = (self.hx, self.hy);
        let y = -x - z;
        let (nx, nz) = if clockwise { (-y, -x) } else { (-z, -y) };
        self.hx = nx;
        self.hy = nz;
    }

    /// `YMove`
    pub fn y_move(&mut self, mut y_change: i32) {
        if self.hx == 0 && self.hy + y_change == 0 {
            y_change = if y_change == -1 { -2 } else { 2 };
        }
        self.hy += y_change;
    }
}

/// `Campaign`. Fields the original adds in `ResetForNewDay` are here from
/// the start.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign_name: String,
    pub campaign_file: String,
    pub player_nation: String,
    pub enemy_nation: String,
    pub map_file: String,
    // what `load_info` reads from the campaign file is read again on
    // loading a saved game, as `LoadGame` does, rather than saved
    #[serde(skip)]
    pub player_veh_list: Vec<String>,
    #[serde(skip)]
    pub mission_activations: Vec<Vec<(String, i32)>>,
    #[serde(skip)]
    pub activation_modifiers: Vec<defs::ActivationModifier>,
    #[serde(skip)]
    pub class_activations: Vec<(String, Vec<(String, i32)>)>,
    #[serde(skip)]
    pub ranks: Vec<defs::Rank>,
    #[serde(skip)]
    pub decorations: Vec<defs::Decoration>,
    #[serde(skip)]
    pub days: Vec<defs::Day>,
    pub over: bool,
    pub exiting: bool,
    pub mouseover: (i32, i32),
    pub color_scheme: Option<String>,

    pub unlimited_tank_selection: bool,
    pub casual_commander: bool,
    pub difficulty: i32,
    pub start_date: usize,

    pub animations: bool,
    pub sounds: bool,
    pub pause_labels: bool,
    pub tutorial_message: bool,

    /// year, month, date
    pub current_date: [i32; 3],

    pub day_vp: i32,
    pub vp: i32,
    pub action_day: bool,
    pub saw_action: bool,
    pub day_in_progress: bool,

    pub scen_res: String,
    pub scen_type: String,

    pub tank_on_offer: String,
    pub selected_crew: Option<usize>,
    pub weather: Weather,

    pub hcbi: i32,
    pub hvap: i32,
    pub apds: i32,

    pub gyro_skill_avail: bool,
    /// Insertion order kept for the stats screen.
    pub stats: Vec<(String, i32)>,
    pub campaign_journal: Vec<String>,
    pub record_day_vp: i32,

    // ResetForNewDay
    pub day_map: Option<CampaignDayMap>,
    pub hour: i32,
    pub minute: i32,
    pub c_map_y: i32,
    pub resupply: bool,
    pub input_mode: String,
    pub selected_node: Option<usize>,
    pub adjacent_nodes: Vec<usize>,
    pub free_check: bool,
    pub messages: Vec<String>,
    pub sunset: bool,
    pub arty_chance: i32,
    pub air_chance: i32,
    pub time_of_last_event: (i32, i32),
    pub quest_active: bool,
    /// (text, minutes or None)
    pub action_list: Vec<(String, Option<i32>)>,
}

impl Campaign {
    pub fn new() -> Self {
        Campaign {
            campaign_name: String::new(),
            campaign_file: String::new(),
            player_nation: String::new(),
            enemy_nation: String::new(),
            map_file: String::new(),
            player_veh_list: Vec::new(),
            mission_activations: Vec::new(),
            activation_modifiers: Vec::new(),
            class_activations: Vec::new(),
            ranks: Vec::new(),
            decorations: Vec::new(),
            days: Vec::new(),
            over: false,
            exiting: false,
            mouseover: (-1, -1),
            color_scheme: None,
            unlimited_tank_selection: false,
            casual_commander: false,
            difficulty: crate::game::DIFFICULTY,
            start_date: 0,
            animations: true,
            sounds: true,
            pause_labels: true,
            tutorial_message: true,
            current_date: [0, 0, 0],
            day_vp: 0,
            vp: 0,
            action_day: false,
            saw_action: false,
            day_in_progress: false,
            scen_res: String::new(),
            scen_type: String::new(),
            tank_on_offer: String::new(),
            selected_crew: None,
            weather: Weather::default(),
            hcbi: 0,
            hvap: 0,
            apds: 0,
            gyro_skill_avail: false,
            stats: Vec::new(),
            campaign_journal: Vec::new(),
            record_day_vp: 0,
            day_map: None,
            hour: 0,
            minute: 0,
            c_map_y: 0,
            resupply: false,
            input_mode: "None".into(),
            selected_node: None,
            adjacent_nodes: Vec::new(),
            free_check: false,
            messages: Vec::new(),
            sunset: false,
            arty_chance: 9,
            air_chance: 7,
            time_of_last_event: (0, 0),
            quest_active: false,
            action_list: Vec::new(),
        }
    }

    /// Fill in what `LoadCampaignInfo` reads from the campaign file.
    pub fn load_info(&mut self, def: &defs::CampaignDef) {
        let d = defs::defs();
        self.campaign_name = def.name.clone();
        self.player_nation = def.player_nation.clone();
        self.enemy_nation = def.enemy_nation.clone();
        if self.player_nation == "USA" {
            self.ranks = d.usa_ranks.clone();
            self.decorations = d.usa_decorations.clone();
        } else if self.player_nation == "CAN" {
            self.ranks = d.ukc_ranks.clone();
            self.decorations = d.ukc_decorations.clone();
        }
        if let Some(m) = &def.map_file {
            self.map_file = m.clone();
        }
        self.player_veh_list = def.player_veh_list.clone();
        self.mission_activations = def.mission_activations.clone();
        // the original appends without clearing; a loaded game reads the
        // file again, so this is cleared first
        self.activation_modifiers = def.activation_modifiers.clone();
        self.class_activations = def.class_activations.clone();
        self.days = def.days.clone();
    }

    /// `GetToday`: the current date's entry in the calendar.
    pub fn today(&self) -> Option<&defs::Day> {
        self.today_index().map(|n| &self.days[n])
    }

    pub fn today_index(&self) -> Option<usize> {
        self.days.iter().position(|d| [d.year, d.month, d.date] == self.current_date)
    }

    /// `SetToday`
    pub fn set_today(&mut self, n: usize) {
        let d = &self.days[n];
        self.current_date = [d.year, d.month, d.date];
    }

    pub fn day_map(&self) -> &CampaignDayMap {
        self.day_map.as_ref().expect("day map")
    }

    pub fn day_map_mut(&mut self) -> &mut CampaignDayMap {
        self.day_map.as_mut().expect("day map")
    }

    /// The player's current area.
    pub fn player_node(&self) -> &MapNode {
        let m = self.day_map();
        &m.nodes[m.player_node]
    }

    pub fn player_node_mut(&mut self) -> &mut MapNode {
        let m = self.day_map_mut();
        let i = m.player_node;
        &mut m.nodes[i]
    }

    /// `GetTerrainDesc`
    pub fn terrain_desc(&self, node: &MapNode) -> &'static str {
        match node.node_type.as_str() {
            "A" => "Farms & Fields",
            "B" => "Fields",
            "C" => "Village",
            "D" => "Woods",
            "F" => "Bocage",
            _ => {
                log::error!("unknown node terrain");
                "Unknown"
            }
        }
    }

    /// `GetRF`: rarity factor of a vehicle type this month; 0 if it was
    /// not about yet.
    pub fn get_rf(&self, vehicle_type: &str) -> i32 {
        const CAMPAIGN_MONTHS: [(i32, i32); 9] =
            [(8, 1944), (9, 1944), (10, 1944), (11, 1944), (12, 1944), (1, 1945), (2, 1945), (3, 1945), (4, 1945)];
        let Some(v) = defs::vehicle_type(vehicle_type) else { return 0 };
        let Some(rarity) = v.stats.list("rarity") else { return 0 };
        let n = if self.current_date == [0, 0, 0] {
            0
        } else {
            let (year, month) = (self.current_date[0], self.current_date[1]);
            if month == 7 && year == 1944 {
                0
            } else {
                CAMPAIGN_MONTHS.iter().position(|&m| m == (month, year)).unwrap_or(0)
            }
        };
        rarity.get(n).copied().unwrap_or(0)
    }

    /// `AddStat`
    pub fn add_stat(&mut self, stat_name: &str, value: i32) {
        match self.stats.iter_mut().find(|(k, _)| k == stat_name) {
            Some((_, v)) => *v += value,
            None => self.stats.push((stat_name.to_string(), value)),
        }
    }

    /// `GetDate`
    pub fn get_date(&self, lookup_date: Option<[i32; 3]>) -> String {
        const MONTHS: [&str; 13] = [
            "", "January", "February", "March", "April", "May", "June", "July", "August", "September",
            "October", "November", "December",
        ];
        let d = lookup_date.unwrap_or(self.current_date);
        let date = d[2].to_string();
        let suffix = match date.as_str() {
            "1" | "21" | "31" => "st",
            "2" | "22" => "nd",
            "3" | "23" => "rd",
            _ => "th",
        };
        format!("{} {date}{suffix}, {}", MONTHS[d[1].clamp(0, 12) as usize], d[0])
    }

    /// `GetSunrise`
    pub fn get_sunrise(&self) -> (i32, i32) {
        const SUNRISE: [(i32, i32); 13] = [
            (0, 0), (7, 45), (7, 15), (6, 15), (5, 15), (5, 0), (5, 0), (5, 0), (5, 0), (5, 30), (6, 30),
            (7, 15), (7, 45),
        ];
        SUNRISE[self.current_date[1].clamp(0, 12) as usize]
    }

    /// `GetSunset`
    pub fn get_sunset(&self) -> (i32, i32) {
        const SUNSET: [(i32, i32); 13] = [
            (0, 0), (16, 30), (17, 30), (18, 0), (19, 0), (19, 15), (19, 15), (19, 15), (19, 15), (18, 15),
            (17, 15), (16, 15), (16, 0),
        ];
        SUNSET[self.current_date[1].clamp(0, 12) as usize]
    }

    /// `ClearAmmo`
    pub fn clear_ammo(&mut self) {
        self.hcbi = 0;
        self.hvap = 0;
        self.apds = 0;
    }

    /// `CheckYOffset`, against the campaign map console size.
    pub fn check_y_offset(&mut self, map_h: i32, window_h: i32) {
        if self.c_map_y < 0 {
            self.c_map_y = 0;
        } else if self.c_map_y > map_h - window_h {
            self.c_map_y = (map_h - window_h).max(0);
        }
    }
}

/// `PlayerTank`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerTank {
    pub unit_type: String,
    pub unit_class: String,
    pub crew: Vec<Crewman>,
    pub stats: Stats,

    pub alive: bool,
    pub swiss_cheese: bool,
    pub name: String,
    pub general_ammo: Ammo,
    pub rr_ammo: Ammo,
    pub ammo_load: String,
    pub ammo_reload: String,
    pub use_rr: bool,
    pub fired_main_gun: bool,
    pub smoke_grenades: i32,
    pub smoke_bombs: i32,
    pub turret_facing: i32,
    pub old_t_facing: i32,
    pub new_facing: i32,
    pub bogged_down: bool,
    pub immobilized: bool,
    pub hull_down: bool,
    pub moving: bool,
    pub lead_tank: bool,
    pub coax_mg_can_fire: bool,
    pub bow_mg_can_fire: bool,
    pub aa_mg_can_fire: bool,
    pub active_mg: i32,
    pub has_rof: bool,
    pub damage_list: Vec<String>,
}

impl PlayerTank {
    /// `PlayerTank(tank_type)`; stats are set by `set_vehicle_stats`.
    pub fn new(tank_type: Option<&str>) -> Self {
        let mut t = PlayerTank {
            unit_type: tank_type.unwrap_or("M4 Turret A").to_string(),
            unit_class: "TANK".into(),
            crew: Vec::new(),
            stats: Stats::default(),
            alive: true,
            swiss_cheese: false,
            name: String::new(),
            general_ammo: Ammo::default(),
            rr_ammo: Ammo::default(),
            ammo_load: "HE".into(),
            ammo_reload: "HE".into(),
            use_rr: true,
            fired_main_gun: false,
            smoke_grenades: 6,
            smoke_bombs: 15,
            turret_facing: 4,
            old_t_facing: 4,
            new_facing: 4,
            bogged_down: false,
            immobilized: false,
            hull_down: false,
            moving: false,
            lead_tank: false,
            coax_mg_can_fire: false,
            bow_mg_can_fire: false,
            aa_mg_can_fire: false,
            active_mg: -1,
            has_rof: false,
            damage_list: Vec::new(),
        };
        t.setup();
        t
    }

    /// `Setup`: a fresh tank, crew kept.
    pub fn setup(&mut self) {
        self.alive = true;
        self.swiss_cheese = false;
        self.name = String::new();
        self.general_ammo.clear();
        self.rr_ammo.clear();
        self.ammo_load = "HE".into();
        self.ammo_reload = "HE".into();
        self.use_rr = true;
        self.fired_main_gun = false;
        self.smoke_grenades = 6;
        self.smoke_bombs = 15;
        self.turret_facing = 4;
        self.old_t_facing = 4;
        self.new_facing = 4;
        self.bogged_down = false;
        self.immobilized = false;
        self.hull_down = false;
        self.moving = false;
        self.lead_tank = false;
        self.coax_mg_can_fire = false;
        self.bow_mg_can_fire = false;
        self.aa_mg_can_fire = false;
        self.active_mg = -1;
        self.has_rof = false;
        self.damage_list.clear();
    }

    /// `Reset`: start of an encounter turn.
    pub fn reset(&mut self) {
        self.old_t_facing = self.turret_facing;
        self.coax_mg_can_fire = false;
        self.bow_mg_can_fire = false;
        self.aa_mg_can_fire = false;
        self.active_mg = -1;
    }

    /// `SelectFirstMG`
    pub fn select_first_mg(&mut self) {
        self.active_mg = if self.coax_mg_can_fire {
            0
        } else if self.bow_mg_can_fire {
            1
        } else if self.aa_mg_can_fire {
            2
        } else {
            -1
        };
    }

    /// `CycleReload`
    pub fn cycle_reload(&mut self) {
        let types: Vec<&str> = AMMO_TYPES.iter().copied().filter(|t| self.general_ammo.has(t)).collect();
        if types.is_empty() {
            return;
        }
        let n = types.iter().position(|t| *t == self.ammo_reload).unwrap_or(0);
        self.ammo_reload = if n >= types.len() - 1 { types[0] } else { types[n + 1] }.to_string();
    }

    pub fn has_damage(&self, d: &str) -> bool {
        self.damage_list.iter().any(|x| x == d)
    }

    /// Index of the crewman in a position (`GetCrewByPosition`).
    pub fn crew_by_position(&self, position: &str) -> Option<usize> {
        self.crew.iter().position(|c| c.position == position)
    }

    /// `crewman.next` and `crewman.prev`: the crew list is the ring.
    pub fn next_crew(&self, i: usize) -> usize {
        (i + 1) % self.crew.len()
    }

    pub fn prev_crew(&self, i: usize) -> usize {
        (i + self.crew.len() - 1) % self.crew.len()
    }
}

/// `Crewman`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Crewman {
    pub name: String,
    pub nickname: String,
    pub hometown: String,
    pub rank_level: usize,
    pub position: String,
    pub order: String,
    pub default_order: String,
    pub hatch: String,
    pub spot: String,
    pub spot_sector: i32,
    pub orders_list: Vec<CrewOrder>,
    pub level: i32,
    pub exp: i32,
    pub skill_pts: i32,
    pub skills: Vec<SkillRecord>,
    pub decorations: Vec<String>,
    pub alive: bool,
    pub light_wound: bool,
    pub serious_wound: bool,
    pub v_serious_wound: bool,
    pub stunned: bool,
    pub unconscious: bool,
    pub bailed_out: bool,
    pub bail_mod: i32,
    pub no_bail: bool,
}

impl Crewman {
    pub fn new() -> Self {
        Crewman {
            name: String::new(),
            nickname: String::new(),
            hometown: String::new(),
            rank_level: 0,
            position: String::new(),
            order: "None".into(),
            default_order: "None".into(),
            hatch: "None".into(),
            spot: "None".into(),
            spot_sector: 4,
            orders_list: Vec::new(),
            level: 1,
            exp: 0,
            skill_pts: 1,
            skills: Vec::new(),
            decorations: Vec::new(),
            alive: true,
            light_wound: false,
            serious_wound: false,
            v_serious_wound: false,
            stunned: false,
            unconscious: false,
            bailed_out: false,
            bail_mod: 0,
            no_bail: false,
        }
    }

    /// `NoActions`
    pub fn no_actions(&self) -> bool {
        !self.alive || self.stunned || self.unconscious
    }

    /// `CurrentOrderOK`
    pub fn current_order_ok(&self) -> bool {
        self.orders_list.iter().any(|o| o.name == self.order)
    }

    /// `UpgradeSkill`
    pub fn upgrade_skill(&mut self, skill_name: &str, skill_level: i32) {
        if let Some(s) = self.skills.iter_mut().find(|s| s.name == skill_name) {
            s.level = skill_level;
        }
    }

    /// `AwardExp`
    pub fn award_exp(&mut self, exp_gain: i32) {
        if !self.alive || self.v_serious_wound {
            return;
        }
        self.exp += exp_gain;
    }

    /// `GetInfo`: name, position, order (in a battle), hatch, spot,
    /// nickname.
    pub fn get_info(&self, in_battle: bool) -> [String; 6] {
        [
            self.name.clone(),
            self.position.clone(),
            if in_battle { self.order.clone() } else { String::new() },
            self.hatch.clone(),
            self.spot.clone(),
            self.nickname.clone(),
        ]
    }
}

/// `EnemyUnit`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnemyUnit {
    pub alive: bool,
    pub map_hex: usize,
    pub x: i32,
    pub y: i32,
    pub animating: bool,
    pub facing: String,
    pub moving: bool,
    pub terrain: String,
    pub pf: bool,
    pub spotted: bool,
    pub identified: bool,
    pub hidden: bool,
    pub spotted_lr: bool,
    pub spotted_tr: bool,
    pub immobile: bool,
    pub shot_at: bool,
    pub fired: bool,
    pub acquired: i32,
    pub acquired_player: i32,
    pub full_apc: bool,
    pub unit_class: String,
    pub unit_type: String,
    pub morale: i32,
    pub pinned: bool,
    pub stunned: bool,
    pub hit_record: Vec<MainGunHit>,
    pub stats: Stats,
    /// Only armoured cars use it; the original adds it to those alone.
    pub spotting_player: bool,
}

impl EnemyUnit {
    /// `EnemyUnit()`; the caller rolls `morale` with `SetMorale`.
    pub fn new(morale: i32) -> Self {
        EnemyUnit {
            alive: true,
            map_hex: 0,
            x: 0,
            y: 0,
            animating: false,
            facing: String::new(),
            moving: false,
            terrain: String::new(),
            pf: false,
            spotted: false,
            identified: false,
            hidden: false,
            spotted_lr: false,
            spotted_tr: false,
            immobile: false,
            shot_at: false,
            fired: false,
            acquired: 0,
            acquired_player: 0,
            full_apc: false,
            unit_class: String::new(),
            unit_type: String::new(),
            morale,
            pinned: false,
            stunned: false,
            hit_record: Vec::new(),
            stats: Stats::default(),
            spotting_player: false,
        }
    }

    pub fn is_class(&self, classes: &[&str]) -> bool {
        classes.contains(&self.unit_class.as_str())
    }
}

/// `Battle`
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Battle {
    pub maphexes: Vec<MapHex>,
    pub smoke_factors: Vec<SmokeFactor>,
    pub messages: Vec<(String, crate::color::Color)>,
    pub enemy_units: Vec<EnemyUnit>,
    pub vp_total: i32,
    pub mouseover: (i32, i32),
    pub tank_type: Option<String>,
    pub spg_type: Option<String>,
    pub at_gun_type: Option<String>,
    pub counterattack: bool,
    pub friendly_ambush: bool,
    pub phase: String,
    pub selected_crew: Option<usize>,
    pub selected_order: Option<usize>,
    pub area_fire: bool,
    pub target: Option<usize>,
    pub trigger_phase: bool,
    pub battle_leadership: bool,
    pub tank_ko_record: [i32; 8],
    pub friendly_ko_record: [i32; 8],
    pub left_behind: [i32; 8],
    pub tanks_lost: i32,
    pub inf_lost: i32,
    pub enemy_reinforcements: i32,
    pub rounds_passed: i32,
    pub result: String,
}

impl Battle {
    /// `Battle(counterattack)`; `res_level` is not used by the original.
    pub fn new(counterattack: bool) -> Self {
        let maphexes = defs::defs()
            .hexes
            .iter()
            .map(|h| {
                let (x, y) = crate::layout::hex2screen(h.0, h.1);
                MapHex { hx: h.0, hy: h.1, rng: h.2, sector: h.3, smoke_factors: 0, x, y }
            })
            .collect();
        Battle {
            maphexes,
            smoke_factors: Vec::new(),
            messages: Vec::new(),
            enemy_units: Vec::new(),
            vp_total: 0,
            mouseover: (-1, -1),
            tank_type: None,
            spg_type: None,
            at_gun_type: None,
            counterattack,
            friendly_ambush: false,
            phase: "None".into(),
            selected_crew: None,
            selected_order: None,
            area_fire: false,
            target: None,
            trigger_phase: false,
            battle_leadership: false,
            tank_ko_record: [0; 8],
            friendly_ko_record: [0; 8],
            left_behind: [0; 8],
            tanks_lost: 0,
            inf_lost: 0,
            enemy_reinforcements: 0,
            rounds_passed: 1,
            result: "Undetermined".into(),
        }
    }

    pub fn hex_at(&self, hx: i32, hy: i32) -> Option<usize> {
        self.maphexes.iter().position(|h| h.hx == hx && h.hy == hy)
    }

    /// `AllEnemiesDead`
    pub fn all_enemies_dead(&self) -> bool {
        self.enemy_units.iter().all(|u| !u.alive)
    }
}

/// `RollAction`: what the to-hit and to-kill windows show.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RollAction {
    pub attacker_unit_type: String,
    pub attacker: String,
    pub attack_type: String,
    pub target_unit_type: String,
    pub target: String,
    pub attacker_unidentified: bool,
    pub target_unidentified: bool,
    pub rng: String,
    pub hit_location: String,
    pub score_req: i32,
    pub auto_ko: bool,
    pub nc: bool,
    pub drm: Vec<(String, i32)>,
    pub total_drm: i32,
    pub roll_req: i32,
    pub roll: i32,
    pub d1: i32,
    pub d2: i32,
    pub result: String,
    pub rof_result: String,
}

impl RollAction {
    /// `CalculateTotalDRM`
    pub fn calculate_total_drm(&mut self) {
        self.total_drm = self.drm.iter().map(|(_, m)| m).sum();
    }
}

/// `IsAdjacent`
pub fn is_adjacent(a: &MapHex, b: &MapHex) -> bool {
    if a.hx == b.hx && a.hy == b.hy {
        return false;
    }
    const DIRECTIONS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
    DIRECTIONS.iter().any(|(dx, dy)| a.hx + dx == b.hx && a.hy + dy == b.hy)
}

/// `GetDistance`: rounded down.
pub fn get_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> i32 {
    ((((x1 - x2) * (x1 - x2) + (y1 - y2) * (y1 - y2)) as f64).sqrt()) as i32
}

/// `GetLine`: Bresenham, from (x1, y1) to (x2, y2) inclusive.
pub fn get_line(mut x1: i32, mut y1: i32, mut x2: i32, mut y2: i32) -> Vec<(i32, i32)> {
    let mut points = Vec::new();
    let issteep = (y2 - y1).abs() > (x2 - x1).abs();
    if issteep {
        std::mem::swap(&mut x1, &mut y1);
        std::mem::swap(&mut x2, &mut y2);
    }
    let mut rev = false;
    if x1 > x2 {
        std::mem::swap(&mut x1, &mut x2);
        std::mem::swap(&mut y1, &mut y2);
        rev = true;
    }
    let deltax = x2 - x1;
    let deltay = (y2 - y1).abs();
    let mut error = deltax / 2;
    let mut y = y1;
    let ystep = if y1 < y2 { 1 } else { -1 };
    for x in x1..=x2 {
        points.push(if issteep { (y, x) } else { (x, y) });
        error -= deltay;
        if error < 0 {
            y += ystep;
            error += deltax;
        }
    }
    if rev {
        points.reverse();
    }
    points
}

/// `GetSectorDistance`
pub fn get_sector_distance(new_f: i32, old_f: i32) -> i32 {
    match (new_f - old_f).abs() {
        4 => 2,
        5 => 1,
        d => d,
    }
}

/// `GetTimeUntil`: hours and minutes from h1:m1 to h2:m2.
pub fn get_time_until(h1: i32, m1: i32, h2: i32, m2: i32) -> (i32, i32) {
    let mut hours = h2 - h1;
    let minutes = if m1 > m2 {
        hours -= 1;
        m2 + 60 - m1
    } else {
        m2 - m1
    };
    (hours, minutes)
}

/// The day map's cell owners as a list of pairs: JSON keys can only be
/// strings.
mod cell_map {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S: Serializer>(map: &HashMap<(i32, i32), usize>, s: S) -> Result<S::Ok, S::Error> {
        let mut pairs: Vec<(&(i32, i32), &usize)> = map.iter().collect();
        pairs.sort();
        pairs.serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<HashMap<(i32, i32), usize>, D::Error> {
        Ok(Vec::<((i32, i32), usize)>::deserialize(d)?.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ends_included_and_ordered() {
        assert_eq!(get_line(0, 0, 3, 1), vec![(0, 0), (1, 0), (2, 1), (3, 1)]);
        assert_eq!(get_line(3, 1, 0, 0).first(), Some(&(3, 1)));
        assert_eq!(get_line(0, 0, 0, 2), vec![(0, 0), (0, 1), (0, 2)]);
    }

    #[test]
    fn sector_distance_wraps() {
        assert_eq!(get_sector_distance(0, 5), 1);
        assert_eq!(get_sector_distance(4, 0), 2);
        assert_eq!(get_sector_distance(1, 4), 3);
    }

    #[test]
    fn ammo_dict() {
        let mut a = Ammo::default();
        assert!(!a.has("HE"));
        a.set("HE", 3);
        a.add("HE", 2);
        assert!(a.has("HE"));
        assert_eq!(a.get("HE"), 5);
        assert_eq!(a.get("WP"), 0);
    }

    #[test]
    fn smoke_rotates_round_the_player() {
        let mut s = SmokeFactor { hx: 0, hy: -1, num_factors: 1.0 };
        for _ in 0..6 {
            s.rotate_position(true);
        }
        assert_eq!((s.hx, s.hy), (0, -1));
    }
}
