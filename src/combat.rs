//! Combat rules and the encounter's action phases ("Encounter Actions and
//! Phases", "Encounter Windows and Animations" in armcom.py).

use crate::color::{self, Color};
use crate::console::Align;
use crate::defs;
use crate::enemy::firing_sound;
use crate::game::*;
use crate::layout::*;
use crate::model::*;
use crate::ui::wrap;

/// `GetArmourStep`: an armour value moved some steps up or down the
/// scale.
pub fn armour_step(base_armour: i32, modifier: i32) -> i32 {
    const ARMOUR_VALUES: [i32; 11] = [0, 1, 2, 3, 4, 6, 8, 11, 14, 18, 26];
    let Some(index) = ARMOUR_VALUES.iter().position(|&a| a == base_armour) else { return base_armour };
    let n = index as i32 + modifier;
    if !(0..=10).contains(&n) { base_armour } else { ARMOUR_VALUES[n as usize] }
}

type Drm = Vec<(String, i32)>;

impl Game {
    /// `GetHitLocation`
    pub fn get_hit_location(&mut self, hull_down: bool) -> &'static str {
        let result = self.roll_1d10();
        if hull_down {
            if result <= 5 { "Turret" } else { "Miss" }
        } else if result <= 4 {
            "Turret"
        } else if result <= 9 {
            "Hull"
        } else {
            "Track"
        }
    }

    /// `CalcTH` with the player's tank firing at unit `i`.
    pub fn calc_th_on_unit(&mut self, i: usize, area_fire: bool, ammo_type: &str) -> (i32, i32, Drm) {
        self.calc_th(i, true, area_fire, ammo_type)
    }

    /// `CalcTH` with unit `i` firing at the player's tank.
    pub fn calc_th_on_player(&mut self, i: usize, area_fire: bool, ammo_type: &str) -> (i32, i32, Drm) {
        self.calc_th(i, false, area_fire, ammo_type)
    }

    /// `CalcTH`: base to-hit number, modifiers, and the roll needed.
    fn calc_th(&mut self, i: usize, player_fires: bool, area_fire: bool, ammo_type: &str) -> (i32, i32, Drm) {
        let rng = self.uhex(i).rng;
        let unit_class = self.u(i).unit_class.clone();
        let target_is_inf = player_fires && ["AT_GUN", "MG", "LW"].contains(&unit_class.as_str());
        let mut base_th = if !area_fire {
            if target_is_inf {
                [8, 5, 2][rng as usize]
            } else {
                [10, 7, 5][rng as usize]
            }
        } else {
            [7, 8, 6][rng as usize]
        };
        let gun = if player_fires { self.tank.stats.str("main_gun").to_string() } else { self.u(i).stats.str("main_gun").to_string() };
        if gun.contains("LL") {
            base_th += [0, 1, 2][rng as usize];
        } else if gun.contains('L') {
            if rng > 0 {
                base_th += 1;
            }
        } else if rng > 0 {
            base_th -= 1;
        }
        if (ammo_type == "WP" || ammo_type == "HCBI") && rng == 0 {
            base_th += 2;
        }
        if gun == "20L" {
            base_th -= [0, 1, 3][rng as usize];
        }

        let mut drm: Drm = Vec::new();
        if player_fires {
            if !self.tank.has_rof {
                let diff = get_sector_distance(self.tank.turret_facing, self.tank.old_t_facing);
                if diff != 0 {
                    drm.push(("Turret has been rotated".into(), diff));
                }
            }
            if let Some(c) = self.crew_pos("Commander") {
                if self.crew(c).hatch == "Shut" && !self.tank.stats.has("vision_cupola") {
                    drm.push(("Commander buttoned up".into(), 1));
                }
            }
            if self.tank.moving {
                if !self.pos_skill_check("Gunner", "Gyrostabilizer") {
                    drm.push(("Firing on the move - Gyrostabilizer skill failed".into(), 4));
                } else {
                    drm.push(("Firing on the move".into(), 2));
                }
            }
            let acquired = self.u(i).acquired;
            if acquired == 1 {
                drm.push(("Target acquired 1".into(), -1));
            } else if acquired == 2 {
                drm.push(("Target acquired 2".into(), -2));
            }
            if acquired < 2 {
                self.um(i).acquired += 1;
            }
        } else {
            let spotter = self.u(i).acquired_player == 0
                && self.bt().enemy_units.iter().enumerate().any(|(n, u)| n != i && u.unit_class == "AC" && u.spotting_player);
            if spotter {
                drm.push(("Target acquired 1 via Spotter".into(), -1));
            } else {
                match self.u(i).acquired_player {
                    1 => drm.push(("Target acquired 1".into(), -1)),
                    2 => drm.push(("Target acquired 2".into(), -2)),
                    _ => {}
                }
            }
            if self.u(i).acquired_player < 2 {
                self.um(i).acquired_player += 1;
            }
            if unit_class == "AT_GUN" {
                let is_88 = self.u(i).unit_type == "88LL";
                let mount = format!("Rotated Facing - 360{} mount", '\u{f8}');
                match self.u(i).facing.as_str() {
                    "Side" => drm.push(if is_88 { (mount, 1) } else { ("Rotated Facing".into(), 2) }),
                    "Rear" => drm.push(if is_88 { (mount, 2) } else { ("Rotated Facing".into(), 3) }),
                    _ => {}
                }
            }
        }

        // the target: a unit when the player fires, the player's tank
        // otherwise
        let (target_vehicle, target_moving, target_size) = if player_fires {
            let u = self.u(i);
            (!u.is_class(&["LW", "MG", "AT_GUN"]), u.moving, u.stats.str("target_size").to_string())
        } else {
            (true, self.tank.moving, self.tank.stats.str("target_size").to_string())
        };
        // the player's gunner's Target Tracking is rolled even when the
        // enemy is firing; kept
        if target_vehicle && target_moving && !self.pos_skill_check("Gunner", "Target Tracking") {
            drm.push(("Vehicle target is moving".into(), 2));
        }
        if target_vehicle {
            match target_size.as_str() {
                "Small" => drm.push(("Small vehicle target".into(), 1)),
                "Large" => drm.push(("Large vehicle target".into(), -1)),
                "Very Large" => drm.push(("Very Large vehicle target".into(), -2)),
                _ => {}
            }
        }
        if player_fires && !area_fire {
            let u = self.u(i);
            if u.unit_class == "AT_GUN" && u.terrain != "Fortification" {
                drm.push(("Emplaced gun target".into(), 2));
            } else {
                match u.terrain.as_str() {
                    "Woods" => drm.push(("Target in Woods".into(), 1)),
                    "Building" => drm.push(("Target in Building".into(), 2)),
                    "Fortification" => drm.push(("Target in Fortification".into(), 3)),
                    _ => {}
                }
            }
        }
        let h = self.uhex(i).clone();
        let smoke = self.get_smoke_factors(0, 0, h.hx, h.hy);
        if smoke > 0 {
            drm.push(("Smoke Factors".into(), smoke * 2));
        }
        if !area_fire && (self.campaign.weather.fog || self.campaign.weather.precip == "Snow") {
            drm.push(("Fog or Falling Snow".into(), 2));
        }
        if player_fires {
            if let Some(c) = self.crew_pos("Commander") {
                if self.crew(c).order == "Direct Main Gun Fire" {
                    if self.crew(c).hatch == "Open" {
                        let m = if self.crew_skill_check(c, "Fire Direction") { -3 } else { -2 };
                        drm.push(("Commander Directing Fire".into(), m));
                    } else if self.tank.stats.has("vision_cupola") {
                        let m = if self.crew_skill_check(c, "Fire Direction") { -2 } else { -1 };
                        drm.push(("Commander Directing Fire".into(), m));
                    }
                }
            }
        }
        let total: i32 = drm.iter().map(|(_, m)| m).sum();
        (base_th, base_th - total, drm)
    }

    /// `CalcTK` with the player's tank hitting unit `i`.
    pub fn calc_tk_on_unit(&mut self, i: usize, facing: &str, ammo_type: &str, critical: bool, area_fire: bool, hit_location: &str) -> (i32, i32, Drm) {
        let target_stats = self.u(i).stats.clone();
        let terrain = self.u(i).terrain.clone();
        let gun = self.tank.stats.str("main_gun").to_string();
        let rng = self.uhex(i).rng;
        self.calc_tk(&target_stats, &terrain, &gun, rng, facing, ammo_type, critical, area_fire, hit_location)
    }

    /// `CalcTK` with unit `i` hitting the player's tank.
    pub fn calc_tk_on_player(&mut self, i: usize, facing: &str, ammo_type: &str, hit_location: &str) -> (i32, i32, Drm) {
        let target_stats = self.tank.stats.clone();
        let gun = self.u(i).stats.str("main_gun").to_string();
        let rng = self.uhex(i).rng;
        self.calc_tk(&target_stats, "", &gun, rng, facing, ammo_type, false, false, hit_location)
    }

    /// `CalcTK`: base to-kill number, modifiers, and the roll needed
    /// (less than).
    #[allow(clippy::too_many_arguments)]
    fn calc_tk(
        &self,
        target: &defs::Stats,
        target_terrain: &str,
        gun: &str,
        rng: i32,
        facing: &str,
        ammo_type: &str,
        critical: bool,
        area_fire: bool,
        hit_location: &str,
    ) -> (i32, i32, Drm) {
        let rng_text = format!("{} Range", ["Close", "Medium", "Long"][rng as usize]);
        let side = facing == "Rear" || facing == "Side";
        let key = match (hit_location == "Hull", side) {
            (true, true) => "hull_side_armour",
            (true, false) => "hull_front_armour",
            (false, true) => "turret_side_armour",
            (false, false) => "turret_front_armour",
        };
        let unarmoured = !target.has(key);
        let mut armour_text = match key {
            "hull_side_armour" => "Hull Side",
            "hull_front_armour" => "Hull Front",
            "turret_side_armour" => "Turret Side",
            _ => "Turret Front",
        }
        .to_string();
        let mut armour_mod = target.int(key);
        if !unarmoured && facing == "Rear" {
            armour_text = if hit_location == "Hull" { "Hull Rear" } else { "Turret Rear" }.into();
            armour_mod = armour_step(armour_mod, -1);
        }
        if !unarmoured {
            armour_text.push_str(" Armour");
        }
        let mut drm: Drm = Vec::new();
        let base_tk;
        if ammo_type == "PF" {
            base_tk = 31;
            drm.push((armour_text, -armour_mod));
        } else if ammo_type == "HE" {
            let (u, a) = match gun {
                "20L" => (6, 3),
                "88L" | "88LL" => (14, 8),
                _ => (12, 7),
            };
            let mut b = if unarmoured { u } else { a };
            if !unarmoured {
                drm.push((armour_text, -armour_mod));
            }
            if unarmoured && critical {
                b *= 2;
            }
            if !critical && area_fire && target_terrain == "Woods" {
                drm.push(("Target in Woods".into(), 1));
            }
            base_tk = b;
        } else if (ammo_type == "HVAP" || ammo_type == "APDS") && !unarmoured {
            let (b, range_mods) = if gun == "76L" {
                (20 + if self.campaign.player_nation == "USA" { 2 } else { 0 }, [2, -2, -5])
            } else {
                (25, [0, 0, -2])
            };
            base_tk = b;
            drm.push((rng_text, range_mods[rng as usize]));
            drm.push((armour_text, -armour_mod));
        } else {
            // AP, and HVAP/APDS against an unarmoured location
            if unarmoured {
                let mut b = match gun {
                    "20L" => 7,
                    "88L" | "88LL" => 10,
                    _ => 9,
                };
                if critical {
                    b *= 2;
                }
                base_tk = b;
            } else {
                let mut b = match gun {
                    "20L" => 6,
                    "50L" => 13,
                    "75" => 14,
                    "75L" | "76L" => 17,
                    "88L" => 20,
                    "75LL" | "76LL" => 23,
                    "88LL" => 27,
                    _ => {
                        log::error!("gun type not found: {gun}");
                        return (2, 2, Vec::new());
                    }
                };
                if critical {
                    b *= 2;
                }
                base_tk = b;
                let range_mods = if gun == "20L" { [1, -1, -3] } else { [0, -1, -2] };
                drm.push((rng_text, range_mods[rng as usize]));
                drm.push((armour_text, -armour_mod));
            }
        }
        let total: i32 = drm.iter().map(|(_, m)| m).sum();
        (base_tk, base_tk + total, drm)
    }

    /// `CalcIFT`: the player's tank against infantry or a truck.
    pub fn calc_ift(&mut self, i: usize, attack_weapon: &str, critical: bool, area_fire: bool, fp: i32, rng: i32) -> (i32, i32, Drm) {
        let u = self.u(i).clone();
        let h = self.uhex(i).clone();
        let mut fp = fp;
        let base_tk = if attack_weapon == "MG" {
            if rng == 8 && h.rng == 1 {
                fp /= 2;
            }
            let inf = u.is_class(&["AT_GUN", "MG", "LW"]);
            // the original leaves the base unset for other firepowers,
            // which never occur in its vehicle data
            match (inf, fp) {
                (true, 1) => 4,
                (true, 2) => 5,
                (true, 4) => 6,
                (false, 1) => 3,
                (false, 2) => 4,
                (false, 4) => 5,
                (true, _) => 4,
                (false, _) => 3,
            }
        } else if attack_weapon == "20L" {
            if critical { 5 } else { 4 }
        } else if attack_weapon == "88L" {
            if critical { 13 } else { 9 }
        } else if critical {
            12
        } else {
            8
        };

        let mut drm: Drm = Vec::new();
        if critical {
            if u.unit_class == "AT_GUN" {
                return (base_tk, 13, drm);
            }
            match u.terrain.as_str() {
                "Woods" => drm.push(("Target in Woods".into(), -1)),
                "Building" => drm.push(("Target in Building".into(), -2)),
                "Fortification" => drm.push(("Target in Fortification".into(), -3)),
                _ => {}
            }
        } else if area_fire {
            if u.unit_class == "AT_GUN" && u.terrain != "Fortification" {
                drm.push(("Emplaced gun target".into(), 2));
            } else {
                match u.terrain.as_str() {
                    "Woods" => drm.push(("Target in Woods".into(), 1)),
                    "Building" => drm.push(("Target in Building".into(), 2)),
                    "Fortification" => drm.push(("Target in Fortification".into(), 3)),
                    _ => {}
                }
            }
        }
        if u.terrain == "Open" && u.moving {
            drm.push(("Target moving in open".into(), -1));
        }
        if ["Mud", "Deep Snow"].contains(&self.campaign.weather.ground.as_str()) && attack_weapon != "MG" {
            drm.push(("HE in Mud or Deep Snow".into(), 1));
        }
        if attack_weapon == "MG" {
            if self.tank.moving {
                drm.push(("Attacker Moving or Pivoting".into(), 1));
            } else if rng == 12 && self.tank.turret_facing != self.tank.old_t_facing {
                drm.push(("Turret has been rotated".into(), 1));
            }
            if u.terrain == "Fortification" {
                drm.push(("Target in fortification".into(), 3));
            } else if u.terrain == "Woods" {
                drm.push(("Target in woods".into(), 1));
            } else if u.terrain == "Building" {
                drm.push(("Target in building".into(), 2));
            } else if u.unit_class == "AT_GUN" {
                drm.push(("Emplaced gun target".into(), 2));
            }
            if let Some(c) = self.crew_pos("Commander") {
                let order = self.crew(c).order.clone();
                if (rng == 12 && order == "Direct Co-ax MG Fire") || (rng == 8 && order == "Direct Bow MG Fire") {
                    let m = if self.crew_skill_check(c, "Fire Direction") { -2 } else { -1 };
                    drm.push(("Commander Directing Fire".into(), m));
                }
            }
            if rng == 8 {
                if let Some(c) = self.crew_pos("Asst. Driver") {
                    if self.crew(c).order == "Fire Bow MG" && self.crew_skill_check(c, "Apprentice Gunner") {
                        drm.push(("Asst. Driver Skill".into(), -1));
                    }
                }
            }
            let smoke = self.get_smoke_factors(0, 0, h.hx, h.hy);
            if smoke > 0 {
                drm.push(("Smoke Factors".into(), smoke * 2));
            }
        }
        let total: i32 = drm.iter().map(|(_, m)| m).sum();
        (base_tk, base_tk - total, drm)
    }

    /// `DisplayRoll`: the to-hit or to-kill window, modifiers drawn one by
    /// one, then the dice.
    pub fn display_roll(&mut self, ra: &RollAction, tk_roll: bool) {
        let animate = self.campaign.animations && !self.scr.is_headless();
        self.darken_screen();
        let m = &mut self.menu_con;
        m.clear();
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.align = Align::Center;
        m.fg = MENU_TITLE_COLOR;
        m.print(MENU_CON_XM, 1, if tk_roll { "To Kill Roll" } else { "To Hit Roll" });
        m.fg = color::WHITE;
        let text = if tk_roll {
            format!("{} hit by {} fired from {}", ra.target, ra.attack_type, ra.attacker)
        } else {
            format!("{} firing {} at {}", ra.attacker, ra.attack_type, ra.target)
        };
        let lines = wrap(&text, 60, "");
        m.print(MENU_CON_XM, 3, &lines[0]);
        if lines.len() > 1 {
            m.print(MENU_CON_XM, 4, &lines[1]);
        }
        if !ra.hit_location.is_empty() {
            m.print(MENU_CON_XM, 5, &format!("Target hit in {}", ra.hit_location));
        }
        let text = if tk_roll {
            if ra.auto_ko {
                "Target automatically destroyed!".to_string()
            } else if ra.nc {
                "No chance to kill target with this attack".to_string()
            } else {
                format!("Base to kill number: {}", ra.score_req)
            }
        } else {
            format!("{} range, base to hit number: {}", ra.rng, ra.score_req)
        };
        m.print(MENU_CON_XM, 6, &text);

        // everything below follows the number of modifiers
        let n = ra.drm.len().max(1) as i32;
        let frame_y = 8;
        let frame_h = n + 5;
        let mut y_after = frame_y + frame_h + 1;
        if !ra.auto_ko && !ra.nc {
            self.menu_con.frame(MENU_CON_XM - 31, frame_y, 63, frame_h, false, false);
            self.menu_con.print(MENU_CON_XM, frame_y + 1, "Dice Roll Modifiers");
            let mut y = frame_y + 3;
            if ra.drm.is_empty() {
                self.menu_con.print(MENU_CON_XM, y, "None");
            } else {
                for (text, num) in &ra.drm {
                    let m = &mut self.menu_con;
                    m.print_ex(MENU_CON_XM - 29, y, Align::Left, text);
                    let t = if *num > 0 { format!("+{num}") } else { num.to_string() };
                    m.print_ex(MENU_CON_XM + 27, y, Align::Right, &t);
                    if animate {
                        self.update_roll_menu(200);
                    }
                    y += 1;
                }
                let m = &mut self.menu_con;
                m.print_ex(MENU_CON_XM - 29, frame_y + frame_h - 2, Align::Left, "Total:");
                let t = if ra.total_drm > 0 { format!("+{}", ra.total_drm) } else { ra.total_drm.to_string() };
                m.print_ex(MENU_CON_XM + 27, frame_y + frame_h - 2, Align::Right, &t);
                if animate {
                    self.update_roll_menu(400);
                }
            }
            let m = &mut self.menu_con;
            if tk_roll {
                m.print(MENU_CON_XM, y_after, &format!("Required to kill: Less than {}", ra.roll_req));
                m.print(MENU_CON_XM, y_after + 1, "To Kill roll (2D6):");
            } else {
                m.print(MENU_CON_XM, y_after, &format!("Required to hit: {} or less", ra.roll_req));
                m.print(MENU_CON_XM, y_after + 1, "To Hit roll (2D6):");
            }
            if animate {
                self.update_roll_menu(900);
            }
            y_after += 2;
        } else {
            y_after = frame_y;
        }

        let mut skip_roll = ra.auto_ko || ra.nc;
        if skip_roll && ra.attack_type.contains("MG") && ra.roll == 12 {
            skip_roll = false;
        }
        if !skip_roll {
            if animate {
                for _ in 0..20 {
                    let text = format!("{}+{}", self.rng.int(1, 6), self.rng.int(1, 6));
                    let m = &mut self.menu_con;
                    m.print(MENU_CON_XM, y_after, "     ");
                    m.print(MENU_CON_XM, y_after, &text);
                    self.play_sound("dice_roll");
                    self.update_roll_menu(45);
                }
            }
            let m = &mut self.menu_con;
            m.print(MENU_CON_XM, y_after, "     ");
            m.print(MENU_CON_XM, y_after, &format!("{}+{}", ra.d1, ra.d2));
            m.print(MENU_CON_XM, y_after + 1, &format!("Total: {}", ra.roll));
            m.print(MENU_CON_XM, y_after + 2, &ra.result);
        }
        if !ra.rof_result.is_empty() {
            self.menu_con.print(MENU_CON_XM, (y_after + 3).min(MENU_CON_HEIGHT - 3), &ra.rof_result);
        }
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 2, "Press Enter to continue");
        self.menu_con.align = Align::Left;
        self.blit_menu();
        self.flush();
        log::info!("roll: {} -> {} {}", ra.attack_type, ra.result, ra.rof_result);
        self.wait_for_enter();
    }

    fn update_roll_menu(&mut self, ms: u64) {
        self.blit_menu();
        self.flush();
        self.wait(ms);
    }

    /// `ResolveCrewFate`: wounds, bailing out and the tank burning, after
    /// it is knocked out or abandoned.
    pub fn resolve_crew_fate(&mut self, hit_location: Option<&str>, sector: Option<i32>, pf: bool, abandoned: bool) {
        self.darken_screen();
        let (x_name, x_wound, x_bail, x_final) = (6, 32, 54, 70);
        let m = &mut self.menu_con;
        m.clear();
        m.align = Align::Center;
        m.fg = color::WHITE;
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.fg = MENU_TITLE_COLOR;
        m.print(MENU_CON_XM, 1, "Tank Knocked Out");
        m.print(MENU_CON_XM, 2, "Crew Wounds and Bail-Out Results");
        m.fg = color::WHITE;
        m.align = Align::Left;
        m.fg = color::LIGHT_GREY;
        m.print(x_name, 4, "Crewman");
        m.print(x_wound, 4, "Initial Wound");
        m.print(x_bail, 4, "Bail Out");
        m.print(x_final, 4, "Final Wound");
        m.hline(x_name - 1, 5, MENU_CON_WIDTH - 2 * x_name + 2);
        m.fg = color::WHITE;

        let rows: Vec<(String, String, String, bool)> = (0..self.tank.crew.len())
            .map(|i| {
                let c = self.crew(i);
                (format!("{} {}", self.crew_rank(i, true), c.name), c.nickname.clone(), Game::wound_text(c), c.no_actions())
            })
            .collect();
        let mut y = 6;
        for (name, nick, wound, _) in &rows {
            let m = &mut self.menu_con;
            if y % 2 == 1 {
                m.bg = ROW_COLOR;
                m.rect(x_name - 1, y, MENU_CON_WIDTH - 2 * x_name + 2, 3, false);
                m.bg = color::BLACK;
            }
            m.print(x_name, y, name);
            if !nick.is_empty() {
                m.print(x_name + 1, y + 1, &format!("\"{nick}\""));
            }
            m.fg = color::RED;
            m.print(x_name + 1, y + 2, wound);
            m.fg = color::WHITE;
            y += 3;
        }
        self.menu_con.align = Align::Center;
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 2, &format!("[{}] to proceed", hl("Enter")));
        self.blit_menu();
        self.flush();
        self.wait_for_enter();
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 2, "                  ");
        self.menu_con.align = Align::Left;

        if !abandoned {
            let mut y = 6;
            for i in 0..self.tank.crew.len() {
                if self.crew(i).alive {
                    match self.crew_take_wound(i, hit_location, sector, false, false) {
                        Some(text) => {
                            self.menu_con.fg = color::RED;
                            for (n, line) in wrap(&text, 18, "").iter().enumerate() {
                                self.menu_con.print(x_wound, y + n as i32, line);
                            }
                            self.menu_con.fg = color::WHITE;
                        }
                        None => self.menu_con.print(x_wound, y, "Not wounded."),
                    }
                }
                self.blit_menu();
                self.flush();
                self.wait(300);
                y += 3;
            }
            self.wait(300);
        }

        let mut y = 6;
        for i in 0..self.tank.crew.len() {
            if self.crew(i).alive {
                let text = self.crew_bail_out(i);
                let name = self.crew(i).name.clone();
                if text == "Passed" {
                    self.write_journal(&format!("{name} bailed out successfully."));
                } else {
                    self.write_journal(&format!("{name} failed to bail out!"));
                    self.menu_con.fg = color::RED;
                }
                self.menu_con.print(x_bail, y, text);
                self.menu_con.fg = color::WHITE;
                self.blit_menu();
                self.flush();
                self.wait(300);
            }
            y += 3;
        }

        let mut crewman_inside = false;
        let mut y = 6;
        for i in 0..self.tank.crew.len() {
            if self.crew(i).bailed_out {
                match self.crew_take_wound(i, None, None, false, false) {
                    Some(text) => {
                        self.menu_con.fg = color::RED;
                        for (n, line) in wrap(&text, 24, "").iter().enumerate() {
                            self.menu_con.print(x_final, y + n as i32, line);
                        }
                        self.menu_con.fg = color::WHITE;
                    }
                    None => self.menu_con.print(x_final, y, "Not wounded."),
                }
                self.blit_menu();
                self.flush();
                self.wait(300);
            } else if self.crew(i).alive {
                crewman_inside = true;
            }
            y += 3;
        }

        if !abandoned {
            let roll = self.roll_1d100();
            let vt = self.tank.stats.str("vehicle_type").to_string();
            let mut target_score = if self.tank.stats.has("wet_stowage") {
                15
            } else if vt.contains("M4A1") {
                75
            } else if vt.contains("M4A3") {
                70
            } else {
                80
            };
            if pf {
                target_score += 5;
            }
            if roll <= target_score && !crewman_inside {
                self.menu_con.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 6, Align::Center, "Your tank burns up, but luckily everyone managed to bail out.");
            } else {
                let (text, result_text) = if roll <= target_score {
                    ("Your tank burns up, killing anyone trapped inside.", "Burns to death")
                } else {
                    ("Luckily your tank does not burn up. Any surviving crewmen still inside are rescued.", "Rescued")
                };
                self.menu_con.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 6, Align::Center, text);
                let mut y = 6;
                for i in 0..self.tank.crew.len() {
                    if !self.crew(i).bailed_out && self.crew(i).alive {
                        if roll <= target_score {
                            self.crewm(i).alive = false;
                            self.menu_con.fg = color::RED;
                        } else {
                            self.crewm(i).bailed_out = true;
                        }
                        self.menu_con.print(x_final, y, result_text);
                        self.menu_con.fg = color::WHITE;
                        self.blit_menu();
                        self.flush();
                        self.wait(300);
                    }
                    y += 3;
                }
            }
        }

        self.menu_con.align = Align::Center;
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 4, "Your combat is over");
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 2, &format!("[{}] to continue", hl("Enter")));
        self.blit_menu();
        self.flush();
        self.wait_for_enter();
        self.menu_con.align = Align::Left;

        if self.check_commander() {
            self.campaign.exiting = true;
        }
    }

    /// `DepleteSmoke`. The original removed spent factors while looping
    /// over the same list, which skipped the one after each; all spent
    /// ones go here.
    pub fn deplete_smoke(&mut self) {
        let rain = self.campaign.weather.precip == "Rain";
        let b = self.btm();
        for s in &mut b.smoke_factors {
            s.num_factors -= if rain { 0.75 } else { 0.5 };
        }
        b.smoke_factors.retain(|s| s.num_factors > 0.0);
    }

    /// `DoSpotting`: each crewman who can spot tries for every enemy unit
    /// in his sectors.
    pub fn do_spotting(&mut self) {
        let mut spot_result = false;
        for ci in 0..self.tank.crew.len() {
            let spot = self.crew(ci).spot.clone();
            let sectors: Vec<i32> = match spot.as_str() {
                "None" => continue,
                "All" => vec![0, 1, 2, 3, 4, 5],
                "Any One Sector" => vec![self.crew(ci).spot_sector],
                "Turret Front" => vec![self.tank.turret_facing],
                "All Except Rear" => vec![0, 2, 3, 4, 5],
                "Tank Front" => vec![4],
                _ => vec![],
            };
            for i in 0..self.bt().enemy_units.len() {
                let u = self.u(i).clone();
                let h = self.uhex(i).clone();
                if !u.alive || !sectors.contains(&h.sector) || u.hidden {
                    continue;
                }
                if u.spotted && u.identified {
                    continue;
                }
                if u.spotted && !u.is_class(&["TANK", "SPG", "AT_GUN"]) {
                    continue;
                }
                if h.rng > 0 && (self.campaign.weather.fog || self.campaign.weather.precip == "Snow") {
                    continue;
                }
                let (_, _, roll) = self.roll_2d6();
                let mut mod_roll = roll;
                let hatch = self.crew(ci).hatch.clone();
                if hatch == "Shut" || hatch == "None" {
                    mod_roll += 2;
                } else if self.crew_skill_check(ci, "Eagle Eyed") {
                    mod_roll -= 2;
                }
                if self.tank.moving {
                    mod_roll += 2;
                }
                if ["Woods", "Building", "Fortification", "Hull Down"].contains(&u.terrain.as_str()) {
                    mod_roll += 2;
                }
                if !u.is_class(&["LW", "MG", "AT_GUN"]) {
                    match u.stats.str("target_size") {
                        "Small" => mod_roll += 1,
                        "Large" => mod_roll -= 1,
                        "Very Large" => mod_roll -= 2,
                        _ => {}
                    }
                }
                if h.rng == 1 {
                    mod_roll -= 1;
                } else if h.rng == 0 {
                    mod_roll -= 2;
                }
                if u.fired {
                    mod_roll -= 2;
                }
                if u.moving {
                    mod_roll -= 2;
                }
                if u.spotted_lr {
                    mod_roll -= 1;
                }
                mod_roll += self.get_smoke_factors(0, 0, h.hx, h.hy) * 2;

                if roll == 12 || mod_roll >= 12 {
                    if u.spotted || u.hidden {
                        continue;
                    }
                    let text = format!("{} is now hidden.", self.unit_desc(i, false));
                    self.um(i).hidden = true;
                    self.unit_label(i, &text);
                    spot_result = true;
                } else if roll == 2 || mod_roll <= 4 {
                    let un = self.um(i);
                    un.spotted = true;
                    if un.is_class(&["TANK", "SPG", "AT_GUN"]) {
                        un.identified = true;
                    }
                    let text = format!("{} spotted!", self.unit_desc(i, false));
                    let (x, y) = self.unit_screen_pos(i);
                    self.show_label(x, y, &text, Some(ci));
                    spot_result = true;
                } else if mod_roll <= 8 {
                    self.um(i).spotted_tr = true;
                    if u.spotted {
                        continue;
                    }
                    self.um(i).spotted = true;
                    let text = format!("{} spotted!", self.unit_desc(i, false));
                    let (x, y) = self.unit_screen_pos(i);
                    self.show_label(x, y, &text, Some(ci));
                    spot_result = true;
                }
                self.update_map_overlay(false);
                self.render_encounter(false);
            }
        }
        if !spot_result {
            self.message("No results from spotting attempts.");
        }
        for u in &mut self.btm().enemy_units {
            u.spotted_lr = false;
            u.fired = false;
            if u.spotted_tr {
                u.spotted_tr = false;
                u.spotted_lr = true;
            }
        }
        self.tank.fired_main_gun = false;
        self.update_map_overlay(false);
    }

    fn order_of(&self, position: &str) -> String {
        self.crew_pos(position).map(|i| self.crew(i).order.clone()).unwrap_or_default()
    }

    /// `SetupFireMGs`
    pub fn setup_fire_mgs(&mut self) -> bool {
        if self.order_of("Commander") == "Fire AA MG" || self.order_of("Loader") == "Fire AA MG" {
            self.tank.aa_mg_can_fire = true;
        }
        if self.order_of("Gunner") == "Fire Co-Axial MG" {
            self.tank.coax_mg_can_fire = true;
        }
        if !self.tank.stats.has("no_asst_driver") && self.order_of("Asst. Driver") == "Fire Bow MG" && !self.tank.hull_down {
            self.tank.bow_mg_can_fire = true;
        }
        self.tank.select_first_mg();
        if self.tank.active_mg == -1 {
            return false;
        }
        self.new_phase("Fire MGs");
        self.select_next_target();
        true
    }

    /// `FireMG`: fire the active MG at the target.
    pub fn fire_mg(&mut self) {
        let Some(t) = self.bt().target else { return };
        let choice = match self.tank.active_mg {
            0 => {
                self.play_sound("coax_mg_firing");
                "Co-ax"
            }
            1 => {
                self.play_sound("bow_mg_firing");
                "Bow"
            }
            2 => {
                self.play_sound("aa_mg_firing");
                "AA"
            }
            _ => return,
        };
        let (tx, ty) = self.unit_screen_pos(t);
        self.mg_animation(MAP_X0 + map_con_x(), MAP_Y0 + map_con_y(), tx, ty);
        let (mg_fp, mg_rng) = match choice {
            "Co-ax" => {
                self.tank.coax_mg_can_fire = false;
                (self.tank.stats.int("co_ax_mg"), 12)
            }
            "Bow" => {
                self.tank.bow_mg_can_fire = false;
                (self.tank.stats.int("bow_mg"), 8)
            }
            _ => {
                self.tank.aa_mg_can_fire = false;
                (self.tank.stats.int("aa_mg"), 8)
            }
        };
        self.tank.select_first_mg();
        let (base_tk, roll_req, drm) = self.calc_ift(t, "MG", false, false, mg_fp, mg_rng);

        let u = self.u(t);
        let h = self.uhex(t);
        let mut ra = RollAction {
            attacker_unit_type: self.tank.unit_type.clone(),
            attacker: format!("{} \"{}\"", self.tank.stats.str("vehicle_type"), self.tank.name),
            attack_type: format!("{choice} MG"),
            target_unidentified: !u.spotted || (u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified),
            target_unit_type: u.unit_type.clone(),
            target: self.unit_desc(t, false),
            rng: ["Close", "Medium", "Long"][h.rng as usize].into(),
            score_req: base_tk,
            drm,
            roll_req,
            ..Default::default()
        };
        ra.calculate_total_drm();
        if roll_req <= 2 {
            ra.nc = true;
        }
        let (d1, d2, roll) = self.roll_2d6();
        ra.d1 = d1;
        ra.d2 = d2;
        ra.roll = roll;
        let desc = self.unit_desc(t, false);
        if d1 == 6 && d2 == 6 {
            ra.result = "The MG has malfunctioned!".into();
        } else if roll < roll_req {
            ra.result = format!("{desc} is destroyed!");
            let text = format!("{desc} was destroyed by MG fire from {}", self.tank.name);
            self.write_journal(&text);
            self.unit_record_ko(t, false, false, false);
            self.um(t).alive = false;
        } else if roll == roll_req || roll == roll_req + 1 {
            if self.u(t).is_class(&["LW", "MG", "AT_GUN"]) {
                self.unit_pin_test(t, true, 0);
                let desc = self.unit_desc(t, false);
                if !self.u(t).alive {
                    ra.result = format!("{desc} is Broken and destroyed!");
                    let text = format!("{desc} was broken and destroyed by MG fire from {}", self.tank.name);
                    self.write_journal(&text);
                } else {
                    ra.result = format!("{desc} is Pinned.");
                }
            } else {
                ra.result = format!("{desc} is unharmed.");
            }
        } else {
            ra.result = format!("{desc} is unharmed.");
        }
        self.display_roll(&ra, true);
        if d1 == 6 && d2 == 6 {
            let d = match choice {
                "Co-ax" => "Co-ax MG Malfunction",
                "Bow" => "Bow MG Malfunction",
                _ => "AA MG Malfunction",
            };
            self.tank_take_damage(Some(d), false, false);
        }
        self.select_next_target();
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// `AttemptRepairs`: crewmen on repair orders, or everything fixable
    /// after an encounter.
    pub fn attempt_repairs(&mut self, post_encounter: bool) {
        if !self.tank.alive || self.tank.swiss_cheese {
            return;
        }
        let list: Vec<String> = self.tank.damage_list.iter().rev().cloned().collect();
        for damage in list {
            if damage.contains("Periscope Broken") {
                if post_encounter {
                    self.popup(&format!("The {damage} has been replaced!"));
                    self.tank.damage_list.retain(|d| *d != damage);
                }
                continue;
            }
            let Some(dt) = defs::defs().damage_type(&damage).cloned() else { continue };
            if dt.repair_score == 0 {
                continue;
            }
            let mut active = None;
            let repair_ok = if dt.auto_repair && post_encounter {
                true
            } else {
                active = self.tank.crew.iter().position(|c| c.order == dt.order);
                active.is_some()
            };
            if !repair_ok {
                continue;
            }
            let (d1, d2, mut roll) = self.roll_2d6();
            if !post_encounter {
                if let Some(c) = active {
                    if self.crew_skill_check(c, "Mechanic") {
                        roll -= 1;
                    }
                }
                if self.order_of("Gunner") == "Help Repair" {
                    roll -= 1;
                }
            } else {
                roll -= 1;
            }
            // the original's "6 and 2" looks meant as double sixes; kept
            if (d1 == 6 && d2 == 2 && dt.break_score != 0) || roll >= dt.break_score {
                self.popup(&format!("The repair attempt failed! {}.", dt.break_result));
                self.tank.damage_list.retain(|d| *d != damage);
                self.tank.damage_list.push(dt.break_result.clone());
            } else if roll > dt.repair_score && !(dt.auto_repair && post_encounter) {
                self.popup(&format!("The {} repair was not successful.", dt.name));
            } else {
                self.popup(&format!("The {} has been repaired!", dt.name));
                self.tank.damage_list.retain(|d| *d != damage);
            }
            if self.battle.is_some() {
                self.update_tank_con();
            }
        }
    }

    /// `SetupMainGun`: true if target selection can go ahead.
    pub fn setup_main_gun(&mut self) -> bool {
        let t = &self.tank;
        if t.has_damage("Main Gun Malfunction") || t.has_damage("Main Gun Broken") || t.has_damage("Gun Sight Broken") {
            return false;
        }
        if self.order_of("Gunner") != "Fire Main Gun" {
            return false;
        }
        if self.tank.moving {
            let gyro = self.crew_pos("Gunner").is_some_and(|g| self.crew(g).skills.iter().any(|s| s.name == "Gyrostabilizer"));
            if !gyro {
                self.popup("Tank is moving and cannot fire main gun.");
                return false;
            }
        }
        if self.tank.ammo_load == "None" {
            self.popup("No shell loaded - order Loader to Change Gun Load.");
            return false;
        }
        match self.tank.ammo_load.as_str() {
            "WP" | "HCBI" => self.btm().area_fire = true,
            "AP" | "HVAP" | "APDS" => self.btm().area_fire = false,
            _ => {}
        }
        self.new_phase("Fire Main Gun");
        self.select_next_target();
        true
    }

    /// `SelectNextTarget`: the next valid target for the active weapon.
    pub fn select_next_target(&mut self) {
        let fog = self.campaign.weather.fog || self.campaign.weather.precip == "Snow";
        let b = self.bt();
        let t = &self.tank;
        let targets: Vec<usize> = (0..b.enemy_units.len())
            .filter(|&n| {
                let u = &b.enemy_units[n];
                let h = &b.maphexes[u.map_hex];
                if !u.alive || u.hidden || !u.spotted {
                    return false;
                }
                if h.rng > 0 && fog {
                    return false;
                }
                if b.phase == "Fire Main Gun" {
                    t.turret_facing == h.sector
                } else if b.phase == "Fire MGs" {
                    if h.rng == 2 || !u.is_class(&["AT_GUN", "MG", "LW", "TRUCK"]) {
                        return false;
                    }
                    match t.active_mg {
                        0 => t.turret_facing == h.sector,
                        1 => h.sector == 4 && !t.hull_down,
                        2 => true,
                        _ => false,
                    }
                } else {
                    false
                }
            })
            .collect();
        let current = b.target;
        let b = self.btm();
        if targets.is_empty() {
            b.target = None;
            return;
        }
        let Some(cur) = current else {
            b.target = Some(targets[0]);
            return;
        };
        // the original walks to the current target, or to the last one
        // if it is no longer in the list, and takes the next
        let idx = targets.iter().position(|&x| x == cur).unwrap_or(targets.len() - 1);
        b.target = Some(if idx < targets.len() - 1 { targets[idx + 1] } else { targets[0] });
    }

    /// `FireMainGun`
    pub fn fire_main_gun(&mut self) {
        let Some(t) = self.bt().target else { return };
        let gunner = self.crew_pos("Gunner");
        let callout = self.roll_1d10();
        if callout == 1 {
            self.player_label("Firing!", gunner);
        }
        if callout == 2 {
            self.player_label("On the way!", gunner);
        }
        if let Some(s) = firing_sound(self.tank.stats.str("main_gun")) {
            self.play_sound(s);
        }
        let (tx, ty) = self.unit_screen_pos(t);
        self.main_gun_animation(MAP_X0 + map_con_x(), MAP_Y0 + map_con_y(), tx, ty);

        let area_fire = self.bt().area_fire;
        let ammo_load = self.tank.ammo_load.clone();
        let (base_th, roll_req, drm) = self.calc_th_on_unit(t, area_fire, &ammo_load);
        self.tank.fired_main_gun = true;
        for (n, u) in self.btm().enemy_units.iter_mut().enumerate() {
            if n != t {
                u.acquired = 0;
            }
        }

        let u = self.u(t);
        let h = self.uhex(t);
        let mut ra = RollAction {
            attacker_unit_type: self.tank.unit_type.clone(),
            attacker: format!("{} \"{}\"", self.tank.stats.str("vehicle_type"), self.tank.name),
            attack_type: format!("{}mm {}", self.tank.stats.str("main_gun").replace('L', ""), ammo_load),
            target_unidentified: !u.spotted || (u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified),
            target_unit_type: u.unit_type.clone(),
            target: self.unit_desc(t, false),
            rng: ["Close", "Medium", "Long"][h.rng as usize].into(),
            score_req: base_th,
            drm,
            roll_req,
            ..Default::default()
        };
        ra.calculate_total_drm();
        let (d1, d2, mut roll) = self.roll_2d6();
        ra.d1 = d1;
        ra.d2 = d2;
        ra.roll = roll;

        let mut hit_result: Option<&str> = None;
        let mut stop_firing = false;
        let gun = self.tank.stats.str("main_gun").to_string();
        if ammo_load == "WP" || ammo_load == "HCBI" {
            if roll == 12 {
                ra.result = "Main Gun Malfunction!".into();
                self.tank.damage_list.push("Main Gun Malfunction".into());
            } else if roll <= roll_req {
                ra.result = "Shot hit target area!".into();
                hit_result = Some("smoke_hit");
                let n = if ammo_load == "WP" { 1.0 } else { 2.0 };
                let hex = self.u(t).map_hex;
                self.place_smoke(hex, n);
                self.paint_map_con();
                if ammo_load == "WP" {
                    self.um(t).hit_record.push(MainGunHit { gun_calibre: gun.clone(), ammo_type: ammo_load.clone(), critical: false, area_fire });
                }
            } else {
                ra.result = "Shot missed target area!".into();
                hit_result = Some("miss");
            }
        } else {
            self.um(t).shot_at = true;
            let weak_spot = roll == 3 && self.pos_skill_check("Gunner", "Knows Weak Spots");
            if ra.roll_req >= 2 && (roll == 2 || weak_spot) {
                ra.result = "Critical Hit!".into();
                stop_firing = true;
                self.um(t).hit_record.push(MainGunHit { gun_calibre: gun.clone(), ammo_type: ammo_load.clone(), critical: true, area_fire });
            } else if roll == 12 {
                ra.result = "Automatic Miss, Main Gun Malfunction!".into();
                self.tank.damage_list.push("Main Gun Malfunction".into());
            } else if roll <= roll_req {
                ra.result = "Shot hit!".into();
                self.um(t).hit_record.push(MainGunHit { gun_calibre: gun.clone(), ammo_type: ammo_load.clone(), critical: false, area_fire });
                hit_result = Some(if ammo_load == "HE" { "he_hit" } else { "ap_hit" });
            } else {
                ra.result = "Shot missed.".into();
                hit_result = Some("miss");
            }
        }

        let old_load = ammo_load.clone();
        self.tank.ammo_load = "None".into();

        if self.tank.has_damage("Main Gun Malfunction") {
            ra.rof_result = "Cannot reload or maintain RoF".into();
            self.update_tank_con();
            self.btm().trigger_phase = true;
        } else {
            let loader = self.crew_pos("Loader");
            let loader_order = self.order_of("Loader");
            if loader_order == "Reload" || loader_order == "Change Gun Load" {
                let reload = self.tank.ammo_reload.clone();
                let mut skill_used = false;
                if self.tank.use_rr && self.tank.general_ammo.get(&reload) > 0 {
                    if let Some(l) = loader {
                        if self.crew_skill_check(l, "Shell Juggler") {
                            self.tank.ammo_load = reload.clone();
                            self.tank.general_ammo.add(&reload, -1);
                            self.update_tank_con();
                            skill_used = true;
                        }
                    }
                }
                if !skill_used {
                    if self.tank.use_rr {
                        if self.tank.rr_ammo.get(&reload) > 0 {
                            self.tank.ammo_load = reload.clone();
                            self.tank.rr_ammo.add(&reload, -1);
                            self.update_tank_con();
                        } else {
                            self.tank.use_rr = false;
                        }
                    }
                    if !self.tank.use_rr && self.tank.general_ammo.get(&reload) > 0 {
                        self.tank.ammo_load = reload.clone();
                        self.tank.general_ammo.add(&reload, -1);
                        self.update_tank_con();
                    }
                }
            }
            if self.order_of("Loader") == "Change Gun Load" {
                ra.rof_result = "Loader changed gun load this round, cannot maintain RoF".into();
                self.btm().trigger_phase = true;
            } else if self.tank.ammo_load == "None" {
                ra.rof_result = if loader_order != "Reload" {
                    "No shell in main gun, cannot maintain RoF".into()
                } else {
                    "No shells of reload type available, cannot reload!".into()
                };
                self.btm().trigger_phase = true;
            } else {
                let load = self.tank.ammo_load.clone();
                if self.bt().area_fire && ["AP", "HVAP", "APDS"].contains(&load.as_str()) {
                    self.btm().area_fire = false;
                    self.update_tank_con();
                } else if !self.bt().area_fire && ["WP", "HCBI"].contains(&load.as_str()) {
                    self.btm().area_fire = true;
                    self.update_tank_con();
                }
                let mut skill_mod = 0;
                if self.tank.use_rr {
                    roll -= 2;
                } else if !self.tank.stats.has("no_asst_driver") {
                    if let Some(a) = self.crew_pos("Asst. Driver") {
                        if self.crew(a).order == "Pass Ammo" {
                            let mut m = -1;
                            if self.crew_skill_check(a, "Shell Tosser") {
                                skill_mod = -2;
                                m = 0;
                            }
                            roll += m;
                        }
                    }
                }
                if self.pos_skill_check("Gunner", "Quick Trigger") && skill_mod > -1 {
                    skill_mod = -1;
                }
                if self.pos_skill_check("Loader", "Fast Hands") && skill_mod > -1 {
                    skill_mod = -1;
                }
                roll += skill_mod;
                if roll <= self.tank.stats.int("rof_num") && !stop_firing {
                    ra.rof_result = "RoF maintained!".into();
                    self.tank.has_rof = true;
                } else {
                    ra.rof_result = "RoF wasn't maintained, end of main gun fire.".into();
                    self.btm().trigger_phase = true;
                }
            }
        }

        self.display_roll(&ra, false);
        self.tank.old_t_facing = self.tank.turret_facing;
        self.render_encounter(false);
        if let Some(r) = hit_result {
            self.hit_animation(r, t);
        }
        self.update_map_overlay(false);
        if self.tank.ammo_load != "None" && old_load != self.tank.ammo_load {
            let text = format!("{} up!", self.tank.ammo_load);
            let loader = self.crew_pos("Loader");
            self.player_label(&text, loader);
        }
        self.render_encounter(false);
    }

    /// `MainGunAnimation`: a shell crossing the map.
    pub fn main_gun_animation(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let line = get_line(x1, y1, x2, y2);
        if line.len() < 3 {
            return;
        }
        for &(x, y) in &line[2..] {
            let Some(old) = self.root.cell(x, y) else { continue };
            let mut c = old;
            c.fg = color::WHITE;
            c.ch = crate::glyph::named::BULLET;
            self.root.set_cell(x, y, c);
            self.flush();
            self.wait(70);
            self.root.set_cell(x, y, old);
        }
        self.flush();
    }

    /// `MGAnimation`: tracer along the line of fire.
    pub fn mg_animation(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let line = get_line(x1, y1, x2, y2);
        if line.len() < 3 {
            return;
        }
        self.update_map_overlay(true);
        self.render_encounter(false);
        let inner = &line[2..line.len() - 1];
        if !inner.is_empty() {
            for _ in 0..20 {
                let (x, y) = *self.rng.choice(inner);
                let Some(old) = self.root.cell(x, y) else { continue };
                let c = self.rng.int(0, 30) as u8;
                let mut cell = old;
                cell.fg = Color(220, 145 + c, 30);
                cell.ch = 249;
                self.root.set_cell(x, y, cell);
                self.flush();
                self.wait(70);
                self.root.set_cell(x, y, old);
            }
        }
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// `ArtyStrikeAnimation`: shell bursts round a point on the screen.
    pub fn arty_strike_animation(&mut self, x: i32, y: i32) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        if self.campaign.sounds {
            self.wait(400);
        }
        let min_x = if self.battle.is_some() { map_con_x() } else { C_MAP_CON_X };
        for _ in 0..10 {
            let x1 = x + self.rng.int(-7, 7);
            let y1 = y + self.rng.int(-4, 4);
            if x1 < min_x || x1 >= sw() - 1 || y1 < 1 || y1 >= sh() {
                continue;
            }
            let Some(mut cell) = self.root.cell(x1, y1) else { continue };
            for (ch, col) in [(249, color::RED), (crate::glyph::named::BULLET, color::RED), (crate::glyph::named::RADIO_UNSET, color::RED), (crate::glyph::named::BLOCK1, color::LIGHT_GREY)] {
                cell.ch = ch;
                cell.fg = col;
                self.root.set_cell(x1, y1, cell);
                self.flush();
                self.wait(40);
            }
        }
        self.blit_con();
        self.flush();
    }

    /// `HitAnimation`
    pub fn hit_animation(&mut self, hit_result: &str, target: usize) {
        if hit_result == "miss" {
            self.play_sound("main_gun_miss");
            return;
        }
        if hit_result == "smoke_hit" {
            self.play_sound("smoke_hit");
            return;
        }
        self.play_sound(hit_result);
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let d = defs::defs();
        let frames = if hit_result == "he_hit" { &d.he_hit_animation } else { &d.ap_hit_animation };
        let (ux, uy) = (self.u(target).x, self.u(target).y);
        let (x, y) = (map_con_x() + ux, map_con_y() + uy);
        let bg = self.map_con.char_bg(ux, uy);
        self.root.set_char_bg(x, y, bg);
        for f in frames {
            if let Some(mut cell) = self.root.cell(x, y) {
                cell.ch = f.0;
                cell.fg = f.1;
                self.root.set_cell(x, y, cell);
            }
            self.flush();
            self.wait(f.2);
        }
        self.flush();
    }

    /// `PlaceSmoke`
    pub fn place_smoke(&mut self, hex: usize, num_factors: f64) {
        let h = self.bt().maphexes[hex].clone();
        self.btm().smoke_factors.push(SmokeFactor { hx: h.hx, hy: h.hy, num_factors });
        self.calculate_smoke_factors();
    }

    /// `CalculateSmokeFactors`
    pub fn calculate_smoke_factors(&mut self) {
        let b = self.btm();
        for h in &mut b.maphexes {
            h.smoke_factors = 0;
        }
        for i in 0..b.maphexes.len() {
            let (hx, hy) = (b.maphexes[i].hx, b.maphexes[i].hy);
            let total: i32 = b.smoke_factors.iter().filter(|s| s.hx == hx && s.hy == hy).map(|s| s.num_factors.ceil() as i32).sum();
            b.maphexes[i].smoke_factors += total;
        }
        self.paint_map_con();
    }

    /// `HandleSmokeGrenades`
    pub fn handle_smoke_grenades(&mut self) {
        for i in 0..self.tank.crew.len() {
            if self.crew(i).order != "Throw Smoke Grenade" {
                continue;
            }
            if self.crew(i).hatch != "Open" {
                let text = format!("{} must have an open hatch to throw a smoke grenade.", self.crew(i).name);
                self.popup(&text);
            } else if self.tank.smoke_grenades == 0 {
                self.popup("No smoke grenades remaining!");
            } else {
                self.tank.smoke_grenades -= 1;
                self.update_tank_con();
                let text = format!("{} throws a smoke grenade!", self.crew(i).name);
                self.message(&text);
                if let Some(h) = self.bt().hex_at(0, 0) {
                    self.place_smoke(h, 0.5);
                }
            }
        }
    }

    /// `HandleSmokeMortar`
    pub fn handle_smoke_mortar(&mut self) {
        for i in 0..self.tank.crew.len() {
            if self.crew(i).order != "Fire Smoke Mortar" {
                continue;
            }
            if self.tank.smoke_bombs == 0 {
                self.popup("No ammo remaining for smoke mortar!");
            } else {
                self.tank.smoke_bombs -= 1;
                self.update_tank_con();
                let text = format!("{} fires smoke from the mortar!", self.crew(i).name);
                self.message(&text);
                let facing = self.tank.turret_facing;
                if let Some(h) = self.bt().maphexes.iter().position(|h| h.rng == 0 && h.sector == facing) {
                    self.place_smoke(h, 1.0);
                }
            }
        }
    }

    /// `Screen2Hex`: the hex under a map cell. The original took the first
    /// hex within 3 cells of the point; with the smaller terminal hexes
    /// this takes the nearest centre, measuring rows double as cells are
    /// twice as tall as wide.
    pub fn screen2hex(&self, x: i32, y: i32) -> Option<usize> {
        let b = self.bt();
        let mut best = None;
        let mut best_d = i32::MAX;
        for (n, h) in b.maphexes.iter().enumerate() {
            let (dx, dy) = (x - h.x, (y - h.y) * 2);
            let d = dx * dx + dy * dy;
            if d < best_d {
                best_d = d;
                best = Some(n);
            }
        }
        if best_d <= 30 { best } else { None }
    }

    /// `GetSmokeFactors`: smoke in the hexes along a line between two hexes.
    pub fn get_smoke_factors(&self, hx1: i32, hy1: i32, hx2: i32, hy2: i32) -> i32 {
        let (x1, y1) = hex2screen(hx1, hy1);
        let (x2, y2) = hex2screen(hx2, hy2);
        let mut hexes: Vec<usize> = Vec::new();
        for (x, y) in get_line(x1, y1, x2, y2) {
            if let Some(h) = self.screen2hex(x, y) {
                if !hexes.contains(&h) {
                    hexes.push(h);
                }
            }
        }
        let b = self.bt();
        hexes.iter().map(|&h| b.maphexes[h].smoke_factors).sum()
    }

    /// `SetupReadyRack`
    pub fn setup_ready_rack(&self) -> bool {
        self.order_of("Loader") == "Restock Ready Rack"
    }

    /// `SetupPivot`
    pub fn setup_pivot(&mut self) -> bool {
        if self.order_of("Driver") != "Pivot Tank" {
            return false;
        }
        if self.tank.bogged_down || self.tank.immobilized {
            self.popup("Tank is immobile and unable to pivot");
            return false;
        }
        self.tank.new_facing = 4;
        true
    }

    /// `PivotTank`: turn the tank, which turns the battlefield round it.
    pub fn pivot_tank(&mut self) {
        let (_, _, roll) = self.roll_2d6();
        if roll == 12 && self.rng.int(1, 6) >= 5 {
            self.popup("Your tank has thrown a track and is immobilized!");
            self.tank.moving = false;
            self.tank.new_facing = 4;
            self.tank.immobilized = true;
            self.update_tank_con();
            self.render_encounter(false);
            return;
        }
        if self.tank.hull_down {
            if roll <= 6 {
                self.popup("Your tank pivots and maintains a Hull Down position.");
            } else {
                self.popup("Your tank pivots but has lost its Hull Down position.");
                self.tank.hull_down = false;
            }
        }
        self.tank.moving = true;
        let (sector_distance, clockwise) = match self.tank.new_facing {
            5 => (1, true),
            0 => (2, true),
            1 => (3, true),
            2 => (2, false),
            _ => (1, false),
        };
        for _ in 0..sector_distance {
            for i in 0..self.bt().enemy_units.len() {
                self.unit_rotate_position(i, clockwise);
            }
            for s in &mut self.btm().smoke_factors {
                s.rotate_position(clockwise);
            }
        }
        self.calculate_smoke_factors();
        self.tank.new_facing = 4;
        self.update_tank_con();
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// Commander directing movement: -2 from an open hatch, -1 from a
    /// vision cupola, one more with the skill.
    fn direct_movement_mod(&mut self) -> i32 {
        let Some(c) = self.crew_pos("Commander") else { return 0 };
        if self.crew(c).order != "Direct Movement" {
            return 0;
        }
        if self.crew(c).hatch == "Open" {
            if self.crew_skill_check(c, "Driver Direction") { -3 } else { -2 }
        } else if self.tank.stats.has("vision_cupola") {
            if self.crew_skill_check(c, "Driver Direction") { -2 } else { -1 }
        } else {
            0
        }
    }

    /// `MoveTank`: the driver's movement order.
    pub fn move_tank(&mut self) {
        let Some(d) = self.crew_pos("Driver") else { return };
        let order = self.crew(d).order.clone();
        if order == "Pivot Tank" {
            return;
        }
        if self.crew(d).no_actions() {
            if self.tank.moving {
                self.popup("Your driver is incapacitated, and your tank rolls to a stop");
                self.tank.moving = false;
                self.update_tank_con();
                self.render_encounter(false);
            }
            return;
        }
        if order == "Stop" && !self.tank.moving {
            return;
        }
        if order == "Stop" {
            if self.tank.moving {
                self.popup("Your tank comes to a stop");
                self.tank.moving = false;
            }
            self.update_tank_con();
            self.render_encounter(false);
            return;
        }
        if order == "Attempt Unbog" && self.tank.bogged_down {
            let (_, _, roll) = self.roll_2d6();
            let mut mod_roll = roll;
            if self.crew(d).hatch == "Open" {
                if self.crew_skill_check(d, "Tough Mudder") {
                    mod_roll -= 2;
                }
            } else {
                mod_roll += 2;
            }
            if self.tank.stats.has("HVSS") {
                mod_roll -= 1;
            }
            mod_roll += self.direct_movement_mod();
            if roll == 12 || mod_roll >= 11 {
                self.popup("You have thrown a track trying to unbog your tank!");
                self.tank.bogged_down = false;
                self.tank.immobilized = true;
                self.update_tank_con();
                let text = self.rng.choice(&defs::defs().crew_talk_thrown_track).clone();
                self.crew_talk(&text, Some(&["Driver"]));
            } else if mod_roll <= 4 {
                self.popup("Your tank is no longer bogged down");
                self.tank.bogged_down = false;
                self.update_tank_con();
            } else {
                self.popup("Driver is unable to unbog tank.");
            }
            return;
        }
        if self.tank.bogged_down || self.tank.immobilized {
            self.message("Tank is unable to move.");
            return;
        }
        self.play_sound("sherman_movement");
        let move_from_stop = !self.tank.moving;
        self.tank.moving = true;
        self.tank.hull_down = false;
        self.update_tank_con();
        self.render_encounter(false);
        for u in &mut self.btm().enemy_units {
            u.acquired = 0;
            u.acquired_player = 0;
            if u.unit_class == "AC" {
                u.spotting_player = false;
            }
        }

        let (_, _, roll) = self.roll_2d6();
        let mut mod_roll = roll + self.direct_movement_mod();
        if self.tank.stats.has("HVSS") {
            mod_roll -= 1;
        }
        if self.crew(d).hatch != "Open" {
            mod_roll += 2;
        }
        if move_from_stop && self.crew_skill_check(d, "Drag Racer") {
            mod_roll -= 1;
        }
        match self.campaign.weather.ground.as_str() {
            "Snow" => mod_roll += 1,
            "Mud" | "Deep Snow" => mod_roll += 2,
            _ => {}
        }
        if roll == 12 || mod_roll >= 12 {
            let d6 = self.rng.int(1, 6);
            if d6 >= 5 {
                self.popup("Your tank has thrown a track and is immobilized!");
                self.tank.moving = false;
                self.tank.immobilized = true;
                self.update_tank_con();
                self.render_encounter(false);
                return;
            } else if d6 >= 3 {
                self.popup("Your tank becomes bogged down!");
                self.tank.moving = false;
                self.tank.bogged_down = true;
                self.update_tank_con();
                self.render_encounter(false);
                return;
            }
        }
        if order == "Forward to Hull Down" {
            if self.crew_skill_check(d, "Eye for Cover") {
                mod_roll -= 2;
            }
            if self.campaign.player_node().node_type == "F" {
                mod_roll -= 2;
            }
            if mod_roll <= 4 {
                self.popup("Your tank moves forward into a hull down position.");
                self.tank.hull_down = true;
                self.update_tank_con();
                self.render_encounter(false);
            } else {
                self.popup("Your tank moves forward but is unable to move into a hull down position.");
            }
        } else if order == "Reverse to Hull Down" {
            if self.crew_skill_check(d, "Eye for Cover") {
                mod_roll -= 1;
            }
            if self.campaign.player_node().node_type == "F" {
                mod_roll -= 2;
            }
            if mod_roll <= 3 {
                self.popup("Your tank moves backward into a hull down position.");
                self.tank.hull_down = true;
                self.update_tank_con();
                self.render_encounter(false);
            } else {
                self.popup("Your tank moves backward but is unable to move into a hull down position.");
            }
        }

        let (_, _, mut roll) = self.roll_2d6();
        if self.tank.stats.str("vehicle_type").contains("M4A3") || self.tank.stats.str("vehicle_class") == "Light Tank" {
            roll -= 2;
        }
        if self.tank.stats.has("duckbills") {
            roll += 2;
        }
        match self.campaign.weather.ground.as_str() {
            "Mud" => roll += 4,
            "Snow" | "Deep Snow" => roll += 2,
            _ => {}
        }
        let (target_score, move_dist) = match order.as_str() {
            "Reverse to Hull Down" => (5, -1),
            "Forward to Hull Down" => (6, 1),
            "Reverse" => (7, -1),
            _ => (8, 1),
        };
        if roll < target_score {
            self.popup("Your tank has moved far enough that enemies are in new positions.");
            for i in 0..self.bt().enemy_units.len() {
                if !self.u(i).alive {
                    continue;
                }
                self.unit_y_move(i, move_dist);
                let u = self.um(i);
                u.spotted = false;
                u.hidden = false;
                u.acquired = 0;
                u.acquired_player = 0;
                if u.unit_class == "AC" {
                    u.spotting_player = false;
                }
            }
            for s in &mut self.btm().smoke_factors {
                s.y_move(move_dist);
            }
            self.calculate_smoke_factors();
            self.update_map_overlay(false);
            self.new_facings();
        } else if roll == target_score {
            self.new_facings();
        } else {
            self.popup("Your tank does not move far enough to affect enemy positions.");
        }
    }

    fn new_facings(&mut self) {
        for i in 0..self.bt().enemy_units.len() {
            if !self.u(i).alive {
                continue;
            }
            if self.unit_set_facing(i) {
                let text = format!("{} now on {} facing", self.unit_desc(i, false), self.u(i).facing);
                self.unit_label(i, &text);
            }
        }
    }

    /// `SetupRotateTurret`
    pub fn setup_rotate_turret(&mut self) -> bool {
        if self.order_of("Gunner") != "Rotate Turret" {
            return false;
        }
        self.tank.old_t_facing = self.tank.turret_facing;
        true
    }

    /// `RotateTurret`: one sector either way.
    pub fn rotate_turret(&mut self, clockwise: bool) {
        if self.tank.has_damage("Turret Traverse Malfunction") || self.tank.has_damage("Turret Traverse Broken") {
            return;
        }
        let f = &mut self.tank.turret_facing;
        *f = if clockwise { (*f + 1) % 6 } else { (*f + 5) % 6 };
        let b = self.bt();
        match b.target {
            Some(t) => {
                if b.phase == "Fire Main Gun" {
                    if self.tank.turret_facing != b.maphexes[b.enemy_units[t].map_hex].sector {
                        self.select_next_target();
                    }
                } else if b.phase == "Fire MGs" && self.tank.active_mg == 0 {
                    self.select_next_target();
                }
            }
            None => self.select_next_target(),
        }
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// `RandomEvent`: one roll on the encounter's random events table.
    pub fn random_event(&mut self) {
        if self.roll_1d6() <= 2 {
            self.message("No Random Event this round.");
            return;
        }
        let ranges: [i32; 7] = match self.campaign.scen_type.as_str() {
            "Advance" => [3, 4, 5, 7, 8, 9, 12],
            "Battle" => [3, 4, 5, 7, 9, 10, 12],
            _ => [3, 5, 6, 7, 9, 12, 0],
        };
        let (_, _, roll) = self.roll_2d6();
        if roll <= ranges[0] {
            if self.bt().all_enemies_dead() {
                self.message("No Random Event this round.");
                return;
            }
            self.popup("Friendly forces conduct Flanking Fire against the enemy.");
            self.render_encounter(false);
            let mut result = false;
            for i in 0..self.bt().enemy_units.len() {
                if !self.u(i).alive {
                    continue;
                }
                if self.unit_friendly_action(i, true, false, false, false) {
                    result = true;
                }
                self.update_map_overlay(false);
                self.render_encounter(false);
            }
            if !result {
                self.popup("No results from Flanking Fire.");
            }
        } else if roll <= ranges[1] {
            if self.bt().all_enemies_dead() {
                self.message("No Random Event this round.");
                return;
            }
            self.popup("Friendly forces conduct artillery fire against the enemy.");
            self.play_sound("arty_firing");
            let mut result = false;
            for i in 0..self.bt().enemy_units.len() {
                if !self.u(i).alive {
                    continue;
                }
                if self.unit_friendly_action(i, false, true, false, false) {
                    result = true;
                }
                self.update_map_overlay(false);
                self.render_encounter(false);
            }
            if !result {
                self.popup("No results from Friendly Artillery.");
            }
        } else if roll <= ranges[2] {
            self.popup("15 minutes of time has passed.");
            self.spend_time(0, 15);
            let text = format!("Time is now {}:{:02}", self.campaign.hour, self.campaign.minute);
            self.write_journal(&text);
            self.render_encounter(false);
        } else if roll <= ranges[3] {
            self.play_sound("german_rifle_fire");
            self.popup("Small-arms fire peppers your tank, threatening any crew member not buttoned up.");
            self.tank_lw_attack();
        } else if roll <= ranges[4] {
            self.popup("Enemy artillery fire rains down on your position.");
            self.play_sound("arty_firing");
            self.arty_strike_animation(MAP_X0 + map_con_x(), MAP_Y0 + map_con_y());
            let result = self.roll_1d10();
            let num_ko = if result <= 6 {
                1
            } else if result <= 9 {
                2
            } else {
                3
            };
            self.popup(&format!("{num_ko} friendly infantry squads are destroyed."));
            self.btm().inf_lost += num_ko;
            self.tank_lw_attack();
        } else if roll <= ranges[5] {
            if self.bt().enemy_reinforcements > 0 {
                let roll = self.roll_1d6();
                if roll != 1 && roll + self.bt().enemy_reinforcements >= 7 {
                    self.message("No Random Event this round.");
                    return;
                }
            }
            self.btm().enemy_reinforcements += 1;
            self.message("Enemy reinforcements have arrived.");
            self.render_encounter(false);
            self.spawn_enemy(None, None);
            self.update_map_overlay(false);
            self.render_encounter(false);
        } else if roll <= ranges[6] {
            if !self.tank.moving {
                self.message("No Random Event this round.");
            } else {
                self.popup("Your battle group has moved into a minefield!");
                self.tank_minefield_attack();
            }
        }
    }

    /// `CheckSpotSectors`: true if any crewman should pick a sector to
    /// watch this turn.
    pub fn check_spot_sectors(&self) -> bool {
        let fog = self.campaign.weather.fog || self.campaign.weather.precip == "Snow";
        let b = self.bt();
        let any = b.enemy_units.iter().any(|u| {
            let h = &b.maphexes[u.map_hex];
            u.alive
                && !u.hidden
                && !(u.spotted && u.identified)
                && !(u.spotted && !u.is_class(&["TANK", "SPG", "AT_GUN"]))
                && !(h.rng > 0 && fog)
        });
        any && self.tank.crew.iter().any(|c| c.spot == "Any One Sector")
    }

    /// `ShowVehicleTypeInfo`: a vehicle's description, in 40 columns.
    pub fn show_vehicle_type_info(con: &mut crate::console::Console, unit_type: &str, x: i32, y: i32, no_image: bool) {
        let Some(vt) = defs::vehicle_type(unit_type) else {
            log::error!("vehicle type not found: {unit_type}");
            return;
        };
        let s = &vt.stats;
        let mut text = s.str("vehicle_type").to_string();
        if s.has("sub_type") {
            text.push_str(&format!(" ({})", s.str("sub_type")));
        }
        if s.has("nickname") {
            text.push_str(&format!(" \"{}\"", s.str("nickname")));
        }
        con.print_ex(x, y, Align::Left, &text);
        con.print_ex(x, y + 1, Align::Left, s.str("vehicle_class"));
        con.fg = color::LIGHT_GREY;
        con.print_ex(x, y + 3, Align::Left, "Main Gun:");
        con.fg = color::WHITE;
        // the original computes "75mm" and throws it away, showing "75"
        let gun = if s.has("main_gun") { s.str("main_gun").to_string() } else { "None".into() };
        con.print_ex(x + 10, y + 3, Align::Left, &gun);
        con.fg = color::LIGHT_GREY;
        con.print_ex(x, y + 5, Align::Left, "Armour:  Front  Side");
        con.print_ex(x, y + 6, Align::Left, "Turret");
        con.print_ex(x, y + 7, Align::Left, "Hull");
        con.fg = color::WHITE;
        let val = |k: &str| if s.has(k) { s.int(k).to_string() } else { "-".into() };
        con.print_ex(x + 11, y + 6, Align::Left, &val("turret_front_armour"));
        con.print_ex(x + 17, y + 6, Align::Left, &val("turret_side_armour"));
        con.print_ex(x + 11, y + 7, Align::Left, &val("hull_front_armour"));
        con.print_ex(x + 17, y + 7, Align::Left, &val("hull_side_armour"));
        if s.has("loader_hatch") {
            con.fg = color::LIGHT_GREY;
            con.print_ex(x, y + 9, Align::Left, "Loader Hatch:");
            con.fg = color::WHITE;
            con.print_ex(x + 14, y + 9, Align::Left, s.str("loader_hatch"));
        }
        let mut ys = 1;
        for (k, label) in [("vision_cupola", "Vision Cupola"), ("smoke_mortar", "Smoke Mortar"), ("wet_stowage", "Wet Stowage"), ("HVSS", "Possible HVSS")] {
            if s.has(k) {
                con.print_ex(x, y + 9 + ys, Align::Left, label);
                ys += 1;
            }
        }
        if s.has("info_text") {
            ys += 1;
            for line in wrap(s.str("info_text"), 34, " ") {
                con.print_ex(x, y + 9 + ys, Align::Left, &line);
                ys += 1;
            }
        }
        if no_image {
            return;
        }
        if s.has("overhead_view") {
            if let Some(img) = crate::xp::by_name(s.str("overhead_view")) {
                // beside the stats, centred top to bottom: the pictures are
                // up to 27 rows tall
                let iy = ((con.h - img.h) / 2).max(1);
                crate::console::Console::blit(&img, 0, 0, img.w, img.h, con, x + 44, iy);
            }
        }
    }

    /// `UnitInfo`: right-click on a unit for its description.
    pub fn unit_info(&mut self, mx: i32, my: i32) {
        if mx < map_con_x() || my < map_con_y() || my >= map_con_y() + MAP_CON_HEIGHT {
            return;
        }
        let (mx, my) = (mx - map_con_x(), my - map_con_y());
        let Some(i) = self.bt().enemy_units.iter().position(|u| u.alive && u.x == mx && u.y == my) else { return };
        let u = self.u(i).clone();
        if u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified {
            return;
        }
        self.darken_screen();
        let desc = self.unit_desc(i, false);
        let m = &mut self.menu_con;
        m.clear();
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.align = Align::Left;
        m.fg = MENU_TITLE_COLOR;
        m.print_ex(MENU_CON_XM, 2, Align::Center, "Unit Info");
        m.fg = color::WHITE;
        m.print_ex(MENU_CON_XM, 4, Align::Center, &desc);
        if u.is_class(&["LW", "MG", "AT_GUN"]) {
            let text = defs::defs().unit_info.get(&u.unit_type).cloned().unwrap_or_default();
            let mut y = 6;
            for line in wrap(&text, 56, " ") {
                m.print(MENU_CON_XM - 28, y, &line);
                y += 1;
            }
        } else {
            Game::show_vehicle_type_info(m, &u.unit_type, 20, 7, false);
        }
        m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 3, Align::Center, "Press ESC to exit");
        self.blit_menu();
        self.flush();
        self.wait_for_escape();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn armour_steps() {
        assert_eq!(armour_step(8, -1), 6);
        assert_eq!(armour_step(0, -1), 0);
        assert_eq!(armour_step(26, 1), 26);
    }
}
