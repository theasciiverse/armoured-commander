//! `EnemyUnit`'s methods, `SpawnEnemy` and `ActivateEnemies`, as `Game`
//! methods taking the unit's index in `battle.enemy_units`.

use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::model::*;

/// `GetFiringSound`: the sound for a gun calibre.
pub fn firing_sound(gun_type: &str) -> Option<&'static str> {
    match gun_type {
        "20L" => Some("20_mm_gun"),
        "50L" => Some("50_mm_gun"),
        "75" | "75L" | "75LL" => Some("75_mm_gun"),
        "76L" | "76LL" => Some("76_mm_gun"),
        "88L" | "88LL" => Some("88_mm_gun"),
        _ => None,
    }
}

impl Game {
    /// Where a unit is on the screen (`self.x+MAP_CON_X, self.y+MAP_CON_Y`).
    pub fn unit_screen_pos(&self, i: usize) -> (i32, i32) {
        let u = self.u(i);
        (u.x + MAP_CON_X, u.y + MAP_CON_Y)
    }

    /// `ShowLabel` at a unit.
    pub fn unit_label(&mut self, i: usize, text: &str) {
        let (x, y) = self.unit_screen_pos(i);
        self.show_label(x, y, text, None);
    }

    /// `ShowLabel` at the player's tank.
    pub fn player_label(&mut self, text: &str, crewman: Option<usize>) {
        self.show_label(MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y, text, crewman);
    }

    /// `SetMorale`
    pub fn roll_morale(&mut self) -> i32 {
        let (_, _, result) = self.roll_2d6();
        if result <= 3 {
            10
        } else if result <= 5 {
            9
        } else if result <= 8 {
            8
        } else {
            7
        }
    }

    /// `PFAttack`: an infantry squad's panzerfaust; true if it fired.
    pub fn unit_pf_attack(&mut self, i: usize) -> bool {
        let u = self.u(i);
        let h = self.uhex(i).clone();
        if !u.pf || h.rng != 0 || u.terrain == "Building" || u.terrain == "Fortification" {
            return false;
        }
        if u.hidden || u.pinned {
            return false;
        }
        let mut roll = self.roll_1d6();
        let [year, month, _] = self.campaign.current_date;
        if year >= 1945 || (year == 1944 && month == 12) {
            roll -= 1;
        }
        if self.tank.moving {
            roll -= 1;
        }
        if self.tank.lead_tank {
            roll -= 1;
        }
        if [0, 1, 2].contains(&h.sector) {
            roll -= 1;
        }
        let target_roll = match self.campaign.scen_type.as_str() {
            "Advance" => 2,
            "Battle" => 3,
            _ => 1,
        };
        if roll > target_roll {
            return false;
        }

        self.um(i).spotted = true;
        self.play_sound("panzerfaust_firing");
        let text = "An infantry squad fires a Panzerfaust at you!";
        self.write_journal(text);
        self.unit_label(i, text);

        let mut ra = RollAction {
            attacker_unit_type: "Light Weapons Infantry".into(),
            attacker: "Infantry Squad".into(),
            attack_type: "Panzerfaust".into(),
            target_unit_type: self.tank.unit_type.clone(),
            target: format!("{} \"{}\"", self.tank.stats.str("vehicle_type"), self.tank.name),
            rng: "Close".into(),
            score_req: 6,
            ..Default::default()
        };
        let mut drm = Vec::new();
        if self.tank.moving {
            drm.push(("Target moving".to_string(), 2));
        }
        let smoke = self.get_smoke_factors(0, 0, h.hx, h.hy);
        if smoke > 0 {
            drm.push(("Smoke Factors".to_string(), smoke * 2));
        }
        ra.drm = drm;
        ra.calculate_total_drm();
        ra.roll_req = ra.score_req - ra.total_drm;

        let (d1, d2, roll) = self.roll_2d6();
        ra.d1 = d1;
        ra.d2 = d2;
        ra.roll = roll;
        let mut hit_location = String::new();
        let mut hit = false;
        if roll == 12 {
            ra.result = "Panzerfaust explodes, squad scattered!".into();
            self.unit_record_ko(i, false, false, false);
            self.um(i).alive = false;
        } else if roll > ra.roll_req {
            ra.result = "Attack missed!".into();
        } else {
            hit_location = self.get_hit_location(self.tank.hull_down).to_string();
            if hit_location == "Miss" {
                ra.result = "The shot misses because your tank is hull down.".into();
            } else if hit_location == "Track" {
                ra.result = "Your tank is hit in the track and is immobilized.".into();
                self.tank.moving = false;
                self.tank.immobilized = true;
            } else {
                hit = true;
                ra.result = format!("The panzerfaust hits your tank in the {hit_location}!");
            }
        }
        self.write_journal(&ra.result.clone());
        self.display_roll(&ra, false);
        self.update_map_overlay(false);
        self.render_encounter(false);

        if hit {
            let facing = self.hit_facing(&hit_location, h.sector);
            let (base_tk, roll_req, drm) = self.calc_tk_on_player(i, &facing, "PF", &hit_location);
            if roll_req < 2 {
                self.player_label("No chance to destroy.", None);
                return true;
            }
            ra.hit_location = hit_location.clone();
            ra.score_req = base_tk;
            ra.drm = drm;
            ra.calculate_total_drm();
            ra.roll_req = roll_req;
            let (d1, d2, roll) = self.roll_2d6();
            ra.d1 = d1;
            ra.d2 = d2;
            ra.roll = roll;
            ra.result = if roll < roll_req {
                "Your tank's armour is penetrated by the hit!".into()
            } else {
                "Your tank is unharmed.".into()
            };
            self.write_journal(&ra.result.clone());
            self.display_roll(&ra, true);
            self.render_encounter(false);
            if roll >= roll_req {
                self.play_sound("armour_save");
            } else {
                let crit = roll == 2 || roll < roll_req / 2;
                self.tank_penetrate(&hit_location, h.sector, "Panzerfaust", crit);
                self.update_tank_con();
                self.render_encounter(false);
            }
        }
        true
    }

    /// Which face of the player's tank a hit from a sector strikes.
    fn hit_facing(&self, hit_location: &str, sector: i32) -> String {
        if hit_location == "Turret" {
            if self.tank.turret_facing == sector {
                "Front"
            } else if get_sector_distance(sector, self.tank.turret_facing) == 3 {
                "Rear"
            } else {
                "Side"
            }
        } else if sector == 4 {
            "Front"
        } else if sector == 1 {
            "Rear"
        } else {
            "Side"
        }
        .to_string()
    }

    /// `PinTest`
    pub fn unit_pin_test(&mut self, i: usize, auto: bool, modifier: i32) {
        if self.unit_morale_test(i, modifier, false) && !auto {
            return;
        }
        if self.u(i).pinned && !self.unit_morale_test(i, 0, true) {
            self.unit_record_ko(i, false, false, false);
            self.um(i).alive = false;
        }
        let u = self.um(i);
        u.pinned = true;
        u.moving = false;
    }

    /// `MoraleTest`
    pub fn unit_morale_test(&mut self, i: usize, modifier: i32, break_test: bool) -> bool {
        let (_, _, mut roll) = self.roll_2d6();
        let u = self.u(i);
        if break_test {
            if u.unit_class == "AT_GUN" && u.terrain != "Fortification" {
                roll -= 2;
            } else if u.terrain == "Woods" {
                roll -= 1;
            } else if u.terrain == "Building" {
                roll -= 2;
            } else if u.terrain == "Fortification" {
                roll -= 3;
            }
        }
        roll != 12 && roll <= u.morale + modifier
    }

    /// `RotatePosition`: move round the player's tank when it pivots.
    pub fn unit_rotate_position(&mut self, i: usize, clockwise: bool) {
        let h = self.uhex(i);
        let (x, z) = (h.hx, h.hy);
        let y = -x - z;
        let (nx, nz) = if clockwise { (-y, -x) } else { (-z, -y) };
        match self.bt().hex_at(nx, nz) {
            Some(n) => {
                self.um(i).map_hex = n;
                let (x, y) = self.unit_char_location(i);
                let u = self.um(i);
                u.x = x;
                u.y = y;
            }
            None => log::error!("could not find hex {nx},{nz}"),
        }
    }

    /// `RecordKO`: note a destroyed unit in the battle record and stats.
    pub fn unit_record_ko(&mut self, i: usize, friendly: bool, left_behind: bool, advance_fire: bool) {
        if !left_behind && friendly {
            let text = format!("{} was destroyed by friendly action", self.unit_desc(i, false));
            self.write_journal(&text);
        }
        let u = self.u(i);
        let index = if u.is_class(&["LW", "MG"]) {
            0
        } else if u.unit_class == "TRUCK" {
            1
        } else if u.is_class(&["APC", "AC"]) {
            2
        } else if u.unit_class == "SPG" {
            3
        } else if u.unit_type == "PzKw IV H" {
            4
        } else if u.unit_type == "PzKw V G" {
            5
        } else if u.unit_type == "PzKw VI E" || u.unit_type == "PzKw VI B" {
            6
        } else if u.unit_class == "AT_GUN" {
            7
        } else {
            log::error!("RecordKO: could not find unit type");
            return;
        };
        let b = self.btm();
        if friendly {
            b.friendly_ko_record[index] += 1;
        } else if left_behind {
            b.left_behind[index] += 1;
        } else {
            b.tank_ko_record[index] += 1;
        }
        if (!friendly && !left_behind) || advance_fire {
            for c in &mut self.tank.crew {
                c.award_exp(1);
            }
        }
        if !left_behind {
            let stat = match index {
                0 => if friendly { "Infantry Destroyed by Allies" } else { "Infantry Destroyed by Player" },
                7 => if friendly { "AT Guns Destroyed by Allies" } else { "AT Guns Destroyed by Player" },
                3..=6 => if friendly { "Tanks & SPGs Destroyed by Allies" } else { "Tanks & SPGs Destroyed by Player" },
                _ => if friendly { "Other Vehicles Destroyed by Allies" } else { "Other Vehicles Destroyed by Player" },
            };
            self.campaign.add_stat(stat, 1);
        }
    }

    /// `SetFacing`: true if the facing changed.
    pub fn unit_set_facing(&mut self, i: usize) -> bool {
        let result = self.roll_1d10();
        let class = self.u(i).unit_class.clone();
        let new_facing = match class.as_str() {
            "SPG" | "AT_GUN" => {
                if result <= 6 { "Front" } else if result <= 9 { "Side" } else { "Rear" }
            }
            "TANK" => {
                if result <= 5 { "Front" } else if result <= 9 { "Side" } else { "Rear" }
            }
            "TRUCK" | "APC" | "AC" => {
                if result <= 3 { "Front" } else if result <= 7 { "Side" } else { "Rear" }
            }
            _ => return false,
        };
        if new_facing != self.u(i).facing {
            self.um(i).facing = new_facing.to_string();
            return true;
        }
        false
    }

    /// `SetTerrain`
    pub fn unit_set_terrain(&mut self, i: usize) {
        const TERRAIN_TYPE: [&str; 4] = ["Hull Down", "Woods", "Building", "Open"];
        const INFANTRY_TERRAIN: [[i32; 4]; 5] = [[0, 2, 8, 10], [0, 3, 5, 10], [0, 1, 6, 10], [0, 6, 7, 10], [0, 4, 5, 10]];
        const VEHICLE_TERRAIN: [[i32; 4]; 5] = [[4, 6, 0, 10], [2, 3, 0, 10], [5, 6, 0, 10], [2, 7, 0, 10], [7, 8, 0, 10]];
        const AREA_TYPES: [&str; 5] = ["A", "B", "C", "D", "F"];
        let node_type = self.campaign.player_node().node_type.clone();
        let table_row = AREA_TYPES.iter().position(|a| *a == node_type).unwrap_or(1);
        let mut result = self.roll_1d10();
        if self.campaign.scen_type == "Counterattack" || self.bt().counterattack {
            result = (result + 2).min(10);
        }
        let is_inf = self.u(i).is_class(&["LW", "MG", "AT_GUN"]);
        let table = if is_inf { &INFANTRY_TERRAIN[table_row] } else { &VEHICLE_TERRAIN[table_row] };
        let n = table.iter().position(|&v| result <= v).unwrap_or(3);
        let mut terrain = TERRAIN_TYPE[n];
        if is_inf {
            if self.campaign.scen_type == "Battle" && (node_type == "B" || node_type == "D") {
                if n == 2 {
                    terrain = "Fortification";
                }
            } else if self.u(i).unit_class == "LW" && result == 10 {
                self.um(i).moving = true;
            }
        } else if (9..=10).contains(&result) {
            self.um(i).moving = true;
        }
        self.um(i).terrain = terrain.to_string();
    }

    /// `GetDesc`: a short description; simpler for a unit that just
    /// appeared.
    pub fn unit_desc(&self, i: usize, new_spawn: bool) -> String {
        let u = self.u(i);
        let class_desc = || -> String {
            match u.unit_class.as_str() {
                "LW" | "MG" => u.unit_type.clone(),
                "AT_GUN" => {
                    if !u.identified {
                        return "Anti-Tank Gun".into();
                    }
                    let text = match u.unit_type.as_str() {
                        "50L" => "PaK 38",
                        "75L" => "PaK 40",
                        "88LL" => "PaK 43",
                        _ => "",
                    };
                    format!("{text} Anti-Tank Gun")
                }
                "TANK" => {
                    if u.identified { format!("{} Tank", u.unit_type) } else { "Tank".into() }
                }
                "SPG" => {
                    if u.identified {
                        format!("{} Self-propelled Gun", u.unit_type)
                    } else {
                        "Self-propelled Gun".into()
                    }
                }
                "TRUCK" => format!("{} Truck", u.unit_type),
                "APC" => format!("{} Armoured Personel Carrier", u.unit_type),
                "AC" => format!("{} Armoured Car", u.unit_type),
                _ => String::new(),
            }
        };
        if new_spawn {
            return class_desc();
        }
        if u.hidden {
            return format!("Hidden {}", class_desc());
        }
        if u.spotted && u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified {
            return format!("Unidentified {}", class_desc());
        }
        if !u.spotted {
            return format!("Unspotted {}", class_desc());
        }
        class_desc()
    }

    /// `YMove`: shift as the player's tank moves forward or back.
    pub fn unit_y_move(&mut self, i: usize, mut y_change: i32) {
        let h = self.uhex(i).clone();
        if h.hx == 0 && h.hy + y_change == 0 {
            y_change = if y_change == -1 { -2 } else { 2 };
        }
        let new_hy = h.hy + y_change;
        if let Some(n) = self.bt().hex_at(h.hx, new_hy) {
            self.um(i).map_hex = n;
            self.um(i).moving = true;
            let (x, y) = self.unit_char_location(i);
            let u = self.um(i);
            u.x = x;
            u.y = y;
            u.hidden = false;
            self.update_map_overlay(false);
            self.render_encounter(false);
            return;
        }
        let text = format!("{} is no longer in the area", self.unit_desc(i, false));
        self.message(&text);
        self.um(i).alive = false;
        self.unit_record_ko(i, false, true, false);
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// `DistMove`: closer (-1) or further (1); true if it moved.
    pub fn unit_dist_move(&mut self, i: usize, dist: i32) -> bool {
        if self.u(i).immobile {
            return false;
        }
        if ["Mud", "Deep Snow"].contains(&self.campaign.weather.ground.as_str()) && self.roll_1d6() <= 3 {
            return false;
        }
        let h = self.uhex(i).clone();
        if h.rng + dist == 3 {
            if self.roll_1d6() <= 3 {
                return false;
            }
            self.um(i).alive = false;
            let text = format!("{} has left the area.", self.unit_desc(i, false));
            self.message(&text);
            self.update_map_overlay(false);
            self.render_encounter(false);
            return true;
        }
        let b = self.bt();
        let move_hexes: Vec<usize> = if h.rng == 0 && dist == -1 {
            (0..b.maphexes.len())
                .filter(|&n| {
                    let m = &b.maphexes[n];
                    m.rng == h.rng && !is_adjacent(&h, m) && !(m.hx == h.hx && m.hy == h.hy)
                })
                .collect()
        } else {
            (0..b.maphexes.len())
                .filter(|&n| {
                    let m = &b.maphexes[n];
                    h.rng + dist == m.rng && is_adjacent(&h, m)
                })
                .collect()
        };
        if move_hexes.is_empty() {
            return false;
        }
        let (old_x, old_y) = (self.u(i).x, self.u(i).y);
        let n = *self.rng.choice(&move_hexes);
        self.um(i).map_hex = n;
        self.unit_move_animation(i, old_x, old_y);
        self.unit_move_effects(i);
        true
    }

    /// `LateralMove`: round the player at the same range.
    pub fn unit_lateral_move(&mut self, i: usize) -> bool {
        if self.u(i).immobile {
            return false;
        }
        if ["Mud", "Deep Snow"].contains(&self.campaign.weather.ground.as_str()) && self.roll_1d6() <= 3 {
            return false;
        }
        let h = self.uhex(i).clone();
        let b = self.bt();
        let move_hexes: Vec<usize> = (0..b.maphexes.len())
            .filter(|&n| {
                let m = &b.maphexes[n];
                h.rng == m.rng && is_adjacent(&h, m)
            })
            .collect();
        if move_hexes.is_empty() {
            return false;
        }
        let (old_x, old_y) = (self.u(i).x, self.u(i).y);
        let n = *self.rng.choice(&move_hexes);
        self.um(i).map_hex = n;
        self.unit_move_animation(i, old_x, old_y);
        self.unit_move_effects(i);
        true
    }

    /// `MoveAnimation`: slide to the new position.
    fn unit_move_animation(&mut self, i: usize, old_x: i32, old_y: i32) {
        let (new_x, new_y) = self.unit_char_location(i);
        let class = self.u(i).unit_class.clone();
        if !["AT_GUN", "MG", "LW"].contains(&class.as_str()) {
            self.play_sound("engine_noise");
        } else if class == "LW" || class == "MG" {
            self.play_sound("infantry_moving");
        }
        if !self.campaign.animations || self.scr.is_headless() {
            let u = self.um(i);
            u.x = new_x;
            u.y = new_y;
            self.update_map_overlay(false);
            self.render_encounter(false);
            return;
        }
        self.um(i).animating = true;
        for (x, y) in get_line(old_x, old_y, new_x, new_y) {
            let u = self.um(i);
            u.x = x;
            u.y = y;
            self.update_map_overlay(false);
            self.render_encounter(false);
            self.wait(50);
        }
        self.um(i).animating = false;
        self.update_map_overlay(false);
        self.render_encounter(false);
    }

    /// `MoveEffects`
    fn unit_move_effects(&mut self, i: usize) {
        let u = self.um(i);
        u.moving = true;
        u.spotted = false;
        u.hidden = false;
        u.acquired = 0;
        u.acquired_player = 0;
        if u.unit_class == "AC" {
            u.spotting_player = false;
        }
        self.unit_set_facing(i);
        self.unit_set_terrain(i);
    }

    /// Fog or falling snow hides anything beyond close range.
    fn unit_out_of_sight(&self, i: usize) -> bool {
        self.uhex(i).rng > 0 && (self.campaign.weather.fog || self.campaign.weather.precip == "Snow")
    }

    /// Smoke between the player and a unit halves a kill chance per factor.
    fn smoke_halved(&self, i: usize, tk_num: i32) -> i32 {
        let h = self.uhex(i);
        let smoke = self.get_smoke_factors(0, 0, h.hx, h.hy);
        if smoke > 0 { (tk_num as f64 * 0.5f64.powi(smoke)).ceil() as i32 } else { tk_num }
    }

    /// `AttackInfantry`: fire at friendly infantry.
    pub fn unit_attack_infantry(&mut self, i: usize) -> bool {
        if self.u(i).hidden || self.unit_out_of_sight(i) {
            return false;
        }
        if !self.u(i).is_class(&["LW", "MG"]) && self.u(i).facing != "Front" {
            if self.u(i).immobile {
                return false;
            }
            let text = format!("{} turns to face you.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.um(i).facing = "Front".into();
            return true;
        }
        let text = format!("{} fires at friendly infantry.", self.unit_desc(i, false));
        self.unit_label(i, &text);
        self.um(i).moving = false;
        let class = self.u(i).unit_class.clone();
        if class == "LW" {
            self.play_sound("german_rifle_fire");
        } else if ["TANK", "SPG", "MG", "APC", "AC"].contains(&class.as_str()) {
            self.play_sound("german_mg_fire");
        }
        let (x, y) = self.unit_screen_pos(i);
        self.mg_animation(x, y, MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y);
        self.um(i).acquired_player = 0;
        self.um(i).fired = true;

        let result = self.roll_1d100();
        if result <= 3 {
            let text = format!("{} destroys a friendly infantry squad.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.btm().inf_lost += 1;
            // the original returns nothing here, which DoAction takes as
            // "try another action"; kept
            return false;
        }
        let rng = self.uhex(i).rng;
        let tk_num = match class.as_str() {
            "TANK" | "SPG" => [65, 40, 10][rng as usize],
            "AC" | "LW" => [30, 20, 3][rng as usize],
            _ => [55, 30, 3][rng as usize],
        };
        let tk_num = self.smoke_halved(i, tk_num);
        if result <= tk_num {
            let text = format!("{} destroys a friendly infantry squad.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.btm().inf_lost += 1;
        } else {
            self.player_label("No effect.", None);
        }
        true
    }

    /// `AttackTank`: fire at a friendly tank.
    pub fn unit_attack_tank(&mut self, i: usize) -> bool {
        if self.u(i).hidden || self.unit_out_of_sight(i) {
            return false;
        }
        if !self.u(i).is_class(&["TANK", "SPG", "AT_GUN"]) {
            return false;
        }
        if self.u(i).facing != "Front" {
            if self.u(i).immobile {
                return false;
            }
            let text = format!("{} turns to face you.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.um(i).facing = "Front".into();
            return true;
        }
        let text = format!("{} fires at a friendly tank.", self.unit_desc(i, false));
        self.unit_label(i, &text);
        self.um(i).moving = false;
        let gun_type = self.u(i).stats.str("main_gun").to_string();
        if let Some(s) = firing_sound(&gun_type) {
            self.play_sound(s);
        }
        let (x, y) = self.unit_screen_pos(i);
        self.main_gun_animation(x, y, MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y);
        self.um(i).acquired_player = 0;
        self.um(i).fired = true;

        let rng = self.uhex(i).rng as usize;
        let mut tk_num = match gun_type.as_str() {
            "50L" => [15, 5, 1][rng],
            "75L" => [52, 40, 22][rng],
            "75LL" => [68, 66, 61][rng],
            "88L" => [68, 63, 43][rng],
            "88LL" => [68, 66, 61][rng],
            _ => {
                self.message("ERROR: Unrecognized gun type in AttackTank()");
                return false;
            }
        };
        if self.u(i).unit_class == "AT_GUN" {
            let is_88 = self.u(i).unit_type == "88LL";
            match self.u(i).facing.as_str() {
                "Side" => tk_num -= if is_88 { 10 } else { 20 },
                "Rear" => tk_num -= if is_88 { 20 } else { 30 },
                _ => {}
            }
            self.um(i).facing = "Front".into();
        }
        let tk_num = self.smoke_halved(i, tk_num);
        let result = self.roll_1d100();
        if result <= tk_num {
            self.play_sound("tank_knocked_out");
            let text = format!("{} destroys a friendly tank.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.btm().tanks_lost += 1;
        } else {
            self.player_label("No effect.", None);
        }
        if self.u(i).hidden || !self.u(i).identified {
            let calibre: String = gun_type.chars().take(2).collect();
            self.crew_talk(&format!("That sounded like a {calibre}mm gun!"), None);
        }
        true
    }

    /// `AttackPlayer`: fire at the player's tank.
    pub fn unit_attack_player(&mut self, i: usize) -> bool {
        if self.u(i).hidden || self.unit_out_of_sight(i) {
            return false;
        }
        if !self.u(i).is_class(&["LW", "MG"]) && self.u(i).facing != "Front" {
            if self.u(i).immobile {
                return false;
            }
            let text = format!("{} turns to face you.", self.unit_desc(i, false));
            self.unit_label(i, &text);
            self.um(i).facing = "Front".into();
            return true;
        }
        if self.u(i).unit_class == "LW" && self.unit_pf_attack(i) {
            // the original returns nothing after a panzerfaust attack, so
            // DoAction rolls this unit another action; kept
            return false;
        }
        let text = format!("{} fires at you!", self.unit_desc(i, false));
        self.unit_label(i, &text);
        self.um(i).moving = false;

        let mut shot_missed = false;
        let mut armour_saved = false;
        let class = self.u(i).unit_class.clone();
        let (x, y) = self.unit_screen_pos(i);
        if ["TANK", "SPG", "AT_GUN"].contains(&class.as_str()) {
            if let Some(s) = firing_sound(self.u(i).stats.str("main_gun")) {
                self.play_sound(s);
            }
            self.main_gun_animation(x, y, MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y);
        } else if class == "LW" {
            self.play_sound("german_rifle_fire");
            self.mg_animation(x, y, MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y);
        } else if ["MG", "APC", "AC"].contains(&class.as_str()) {
            self.play_sound("german_mg_fire");
            self.mg_animation(x, y, MAP_X0 + MAP_CON_X, MAP_Y0 + MAP_CON_Y);
        }
        self.um(i).fired = true;

        if !["TANK", "SPG", "AT_GUN"].contains(&class.as_str()) {
            self.tank_lw_attack();
            return true;
        }

        let (base_th, roll_req, drm) = self.calc_th_on_player(i, false, "AP");
        if class == "AT_GUN" {
            self.um(i).facing = "Front".into();
        }
        if roll_req < 2 {
            self.player_label("The attack cannot hit you.", None);
            return true;
        }

        let u = self.u(i);
        let h = self.uhex(i).clone();
        let mut ra = RollAction {
            attacker_unit_type: u.unit_type.clone(),
            attacker_unidentified: !u.spotted || (u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified),
            attacker: self.unit_desc(i, false),
            attack_type: format!("{}mm AP", u.stats.str("main_gun").replace('L', "")),
            target_unit_type: self.tank.unit_type.clone(),
            target: format!("{} \"{}\"", self.tank.stats.str("vehicle_type"), self.tank.name),
            rng: ["Close", "Medium", "Long"][h.rng as usize].into(),
            score_req: base_th,
            drm,
            roll_req,
            ..Default::default()
        };
        ra.calculate_total_drm();

        let (d1, d2, roll) = self.roll_2d6();
        ra.d1 = d1;
        ra.d2 = d2;
        ra.roll = roll;

        if roll <= roll_req {
            let hit_location = self.get_hit_location(self.tank.hull_down).to_string();
            if hit_location == "Miss" {
                ra.result = "The shot misses because your tank is hull down.".into();
            } else if hit_location == "Track" {
                ra.result = "Your tank is hit in the track and is immobilized.".into();
                self.tank.moving = false;
                self.tank.immobilized = true;
                if let Some(d) = self.crew_pos("Driver") {
                    self.crew_build_orders_list(d, false);
                }
            } else {
                ra.result = format!("Your tank is hit in the {hit_location}");
                self.display_roll(&ra, false);
                self.update_tank_con();
                self.render_encounter(false);

                let facing = self.hit_facing(&hit_location, h.sector);
                let (base_tk, roll_req, drm) = self.calc_tk_on_player(i, &facing, "AP", &hit_location);
                if roll_req < 2 {
                    self.player_label("No chance to destroy.", None);
                    return true;
                }
                ra.hit_location = hit_location.clone();
                ra.score_req = base_tk;
                ra.drm = drm;
                ra.calculate_total_drm();
                ra.roll_req = roll_req;
                let (d1, d2, roll) = self.roll_2d6();
                ra.d1 = d1;
                ra.d2 = d2;
                ra.roll = roll;
                if roll < roll_req {
                    ra.result = "Your tank's armour is penetrated by the hit!".into();
                } else {
                    ra.result = "Your tank is unharmed.".into();
                }
                self.display_roll(&ra, true);
                if roll >= roll_req {
                    self.play_sound("armour_save");
                } else {
                    self.play_sound("ap_hit");
                }
                self.update_tank_con();
                self.render_encounter(false);
                if roll < roll_req {
                    let crit = roll == 2 || roll < roll_req / 2;
                    let gun = self.u(i).stats.str("main_gun").to_string();
                    self.tank_penetrate(&hit_location, h.sector, &gun, crit);
                } else {
                    for c in 0..self.tank.crew.len() {
                        if self.crew_stun_check(c, 10) {
                            let text = format!("{} is Stunned from the impact!", self.crew(c).name);
                            self.popup(&text);
                        }
                    }
                }
                // the original returns here, before its crew talk after a
                // saving throw; kept
                let _ = armour_saved;
                return true;
            }
        } else {
            ra.result = "Shot misses!".into();
            shot_missed = true;
        }

        self.display_roll(&ra, false);
        self.update_tank_con();
        self.render_encounter(false);

        if shot_missed {
            self.play_sound("main_gun_miss");
            if self.u(i).hidden || !self.u(i).identified {
                // the original took one character here ("7mm"); two, as
                // in AttackTank
                let calibre: String = self.u(i).stats.str("main_gun").chars().take(2).collect();
                self.crew_talk(&format!("That sounded like a {calibre}mm gun!"), None);
            }
        }
        let roll = self.roll_1d10();
        let d = defs::defs();
        if armour_saved && roll <= 4 {
            let text = self.rng.choice(&d.crew_talk_armour_saved).clone();
            self.crew_talk(&text, None);
        } else if shot_missed && roll <= 2 {
            let text = self.rng.choice(&d.crew_talk_shot_missed).clone();
            self.crew_talk(&text, None);
        }
        armour_saved = false;
        let _ = armour_saved;
        true
    }

    /// `ResolveHits`: the main gun hits recorded against this unit.
    pub fn unit_resolve_hits(&mut self, i: usize) {
        let vehicle = !self.u(i).is_class(&["LW", "MG", "AT_GUN"]);
        let hits = std::mem::take(&mut self.um(i).hit_record);
        for gun_hit in hits {
            if !self.u(i).alive {
                continue;
            }
            if gun_hit.ammo_type == "WP" && vehicle {
                continue;
            }
            let text = format!(
                "Resolving {}{} hit against {}",
                if gun_hit.critical { "critical " } else { "" },
                gun_hit.ammo_type,
                self.unit_desc(i, false)
            );
            self.message(&text);

            if gun_hit.ammo_type == "WP" {
                self.unit_pin_test(i, false, 0);
                let desc = self.unit_desc(i, false);
                if !self.u(i).alive {
                    self.unit_label(i, &format!("{desc} surrenders!"));
                } else if self.u(i).pinned {
                    self.unit_label(i, &format!("{desc} is pinned!"));
                } else {
                    self.unit_label(i, "WP has no additional effect.");
                }
                continue;
            }

            let mut hit_location = String::new();
            let (base_tk, roll_req, drm) = if !vehicle {
                if ["AP", "HVAP", "APDS"].contains(&gun_hit.ammo_type.as_str()) {
                    self.unit_label(i, "AP has no effect against infantry targets.");
                    continue;
                }
                self.calc_ift(i, &gun_hit.gun_calibre, gun_hit.critical, gun_hit.area_fire, 0, 0)
            } else {
                hit_location = self.get_hit_location(self.u(i).terrain == "Hull Down").to_string();
                if hit_location == "Miss" {
                    let text = format!("{} is hulldown and was unharmed by hit.", self.unit_desc(i, false));
                    self.popup(&text);
                    continue;
                } else if hit_location == "Track" {
                    let desc = self.unit_desc(i, false);
                    self.popup(&format!("{desc} is immobilized."));
                    let text = format!("{desc} was immobilized by a {} hit from {}", gun_hit.ammo_type, self.tank.name);
                    self.write_journal(&text);
                    let u = self.um(i);
                    u.immobile = true;
                    u.moving = false;
                    if u.unit_class == "APC" && u.full_apc {
                        self.unit_dismount_infantry(i, true);
                    }
                    continue;
                }
                let facing = self.u(i).facing.clone();
                self.calc_tk_on_unit(i, &facing, &gun_hit.ammo_type, gun_hit.critical, gun_hit.area_fire, &hit_location)
            };

            let u = self.u(i);
            let h = self.uhex(i);
            let mut ra = RollAction {
                attacker_unit_type: self.tank.unit_type.clone(),
                attacker: format!("{} \"{}\"", self.tank.stats.str("vehicle_type"), self.tank.name),
                attack_type: format!("{}mm {}", gun_hit.gun_calibre, gun_hit.ammo_type),
                target_unidentified: !u.spotted || (u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified),
                target_unit_type: u.unit_type.clone(),
                target: self.unit_desc(i, false),
                rng: ["Close", "Medium", "Long"][h.rng as usize].into(),
                hit_location: if vehicle { hit_location.clone() } else { String::new() },
                score_req: base_tk,
                drm,
                roll_req,
                ..Default::default()
            };
            ra.calculate_total_drm();

            if roll_req <= 2 || roll_req > 12 {
                if roll_req <= 2 {
                    ra.nc = true;
                } else {
                    ra.auto_ko = true;
                }
                self.display_roll(&ra, true);
                if roll_req > 12 {
                    let text = format!("{} was destroyed by a {} hit from {}", self.unit_desc(i, false), gun_hit.ammo_type, self.tank.name);
                    self.write_journal(&text);
                    self.unit_record_ko(i, false, false, false);
                    self.um(i).alive = false;
                    if self.u(i).unit_class == "APC" && self.u(i).full_apc {
                        self.unit_dismount_infantry(i, true);
                    }
                    self.update_map_overlay(false);
                } else if ["AP", "HVAP", "APDS"].contains(&gun_hit.ammo_type.as_str()) && vehicle {
                    self.play_sound("armour_save");
                }
                self.render_encounter(false);
                continue;
            }

            let (d1, d2, roll) = self.roll_2d6();
            ra.d1 = d1;
            ra.d2 = d2;
            ra.roll = roll;
            let desc = self.unit_desc(i, false);
            if roll < roll_req {
                ra.result = format!("{desc} is destroyed!");
                let text = format!("{desc} was destroyed by a {} hit from {}", gun_hit.ammo_type, self.tank.name);
                self.write_journal(&text);
                self.unit_record_ko(i, false, false, false);
                self.um(i).alive = false;
                if self.u(i).unit_class == "APC" && self.u(i).full_apc {
                    self.unit_dismount_infantry(i, true);
                }
            } else if roll == roll_req || (!vehicle && roll == roll_req + 1) {
                if !vehicle {
                    self.unit_pin_test(i, true, 0);
                    let desc = self.unit_desc(i, false);
                    if !self.u(i).alive {
                        ra.result = format!("{desc} is Broken and destroyed!");
                        let text = format!("{desc} was broken and destroyed by a {} hit from {}", gun_hit.ammo_type, self.tank.name);
                        self.write_journal(&text);
                    } else {
                        ra.result = format!("{desc} is Pinned.");
                    }
                } else if self.u(i).stunned {
                    if !self.unit_morale_test(i, 0, true) {
                        ra.result = format!("{desc} is Stunned again and abandoned!");
                        let text = format!("{desc} was Stunned by a {} hit from {} and abandoned", gun_hit.ammo_type, self.tank.name);
                        self.write_journal(&text);
                        self.unit_record_ko(i, false, false, false);
                        self.um(i).alive = false;
                    } else {
                        ra.result = format!("{desc} remains Stunned.");
                    }
                } else {
                    ra.result = format!("{desc} is Stunned!");
                    let u = self.um(i);
                    u.stunned = true;
                    u.moving = false;
                    if u.unit_class == "AC" {
                        u.spotting_player = false;
                    }
                }
            } else {
                ra.result = format!("{desc} is unharmed");
                if !vehicle {
                    self.unit_pin_test(i, false, roll - roll_req);
                    if self.u(i).pinned {
                        ra.result.push_str(" but pinned");
                    }
                }
                ra.result.push('.');
            }
            self.display_roll(&ra, true);
            if vehicle && !self.u(i).alive {
                self.play_sound("tank_knocked_out");
            } else if vehicle && ["AP", "HVAP", "APDS"].contains(&gun_hit.ammo_type.as_str()) {
                self.play_sound("armour_save");
            }
            self.update_map_overlay(false);
            self.render_encounter(false);
        }
    }

    /// `FriendlyAction`: the rest of the battlegroup against this unit.
    pub fn unit_friendly_action(&mut self, i: usize, flanking_fire: bool, artillery: bool, air_strike: bool, advance_fire: bool) -> bool {
        let (_, _, roll) = self.roll_2d6();
        let destroyed = |g: &mut Game| {
            if g.u(i).is_class(&["TANK", "SPG"]) {
                g.play_sound("tank_knocked_out");
            }
            let text = format!("{} is destroyed by friendly action!", g.unit_desc(i, false));
            g.unit_label(i, &text);
            g.unit_record_ko(i, true, false, advance_fire);
            g.um(i).alive = false;
            g.update_map_overlay(false);
            g.render_encounter(false);
        };
        if roll == 2 {
            destroyed(self);
            return true;
        }
        let u = self.u(i).clone();
        let h = self.uhex(i).clone();
        let b = self.bt();
        let mut mod_roll = roll;
        if flanking_fire {
            mod_roll -= 1;
        }
        if artillery && u.is_class(&["LW", "MG", "AT_GUN"]) && u.terrain == "Woods" {
            mod_roll -= 1;
        }
        if u.is_class(&["LW", "MG"]) {
            if 2 < b.inf_lost && b.inf_lost < 4 {
                mod_roll += 1;
            } else if b.inf_lost >= 4 {
                mod_roll += 2;
            }
        }
        if u.is_class(&["TANK", "SPG", "APC", "AC", "TRUCK"]) {
            if !air_strike {
                if 1 < b.tanks_lost && b.tanks_lost < 3 {
                    mod_roll += 1;
                } else if b.tanks_lost >= 3 {
                    mod_roll += 2;
                }
            } else {
                mod_roll -= 1;
            }
        }
        if (artillery || advance_fire) && u.is_class(&["TANK", "SPG", "APC", "AC"]) {
            mod_roll += 2;
        }
        if advance_fire {
            mod_roll += [1, 2, 4][h.rng as usize];
        }
        if !air_strike && !artillery {
            let total_smoke: i32 =
                b.maphexes.iter().filter(|m| m.rng == h.rng && m.sector == h.sector).map(|m| m.smoke_factors).sum();
            mod_roll += total_smoke.max(0);
        }
        if air_strike {
            mod_roll += 2;
        }
        if self.tank.has_damage("Radio Malfunction") || self.tank.has_damage("Radio Broken") {
            mod_roll += 2;
        }
        if artillery && (self.campaign.player_nation == "CAN" || self.campaign.player_nation == "UK") {
            mod_roll -= 2;
        }
        let tk_num = match u.unit_class.as_str() {
            "TRUCK" => 7,
            "APC" => 6,
            "AC" | "LW" | "MG" => 5,
            "AT_GUN" => 4,
            _ => match u.unit_type.as_str() {
                "Marder II" | "Marder III H" => 6,
                "PzKw IV H" | "STuG III G" => 5,
                "PzKw V G" | "JgdPzKw IV" | "JgdPz 38(t)" => 4,
                "PzKw VI E" | "PzKw VI B" => 3,
                _ => {
                    log::error!("FriendlyAction: unrecognized unit type");
                    return false;
                }
            },
        };
        if mod_roll <= tk_num {
            destroyed(self);
            return true;
        }

        let mut mod_roll = roll;
        if u.is_class(&["TANK", "SPG", "AT_GUN"]) {
            mod_roll += 1;
        }
        if artillery {
            mod_roll += 1;
        }
        if air_strike || advance_fire {
            if mod_roll >= 11 && u.is_class(&["LW", "MG", "AT_GUN"]) {
                let text = format!("{} is pinned.", self.unit_desc(i, false));
                self.unit_label(i, &text);
                self.um(i).pinned = true;
                self.render_encounter(false);
                return true;
            }
        } else {
            if u.is_class(&["LW", "MG"]) {
                mod_roll -= 2;
            }
            if mod_roll >= 11 && h.smoke_factors == 0 {
                let text = format!("{} is hit with smoke from friendly units.", self.unit_desc(i, false));
                self.unit_label(i, &text);
                let hex = self.u(i).map_hex;
                self.place_smoke(hex, 1.0);
                self.render_encounter(false);
                return true;
            }
        }
        false
    }

    /// `DismountInfantry`: an APC unloads a squad.
    pub fn unit_dismount_infantry(&mut self, i: usize, under_fire: bool) {
        self.um(i).full_apc = false;
        let text = format!("{} dismounts infantry!", self.unit_desc(i, false));
        self.unit_label(i, &text);
        let hex = self.u(i).map_hex;
        let class = if self.roll_1d6() <= 4 { "LW" } else { "MG" };
        if let Some(n) = self.spawn_enemy(Some(class), Some(hex)) {
            if under_fire {
                self.unit_pin_test(n, true, 0);
            }
        }
    }

    /// `GetCharLocation`: a free spot inside the unit's hex. The original
    /// compared `abs(unit.x + x)` where it meant the difference, so units
    /// could land on top of each other; fixed. Rows are those inside the
    /// smaller terminal hex, with room for the terrain marks either side.
    pub fn unit_char_location(&mut self, i: usize) -> (i32, i32) {
        if !self.u(i).alive {
            return (-1, -1);
        }
        let h = self.uhex(i).clone();
        for _ in 0..100 {
            let y_mod = self.rng.int(-1, 1);
            let (lo, hi) = if y_mod == -1 { (-1, 2) } else { (-2, 3) };
            let x_mod = self.rng.int(lo, hi);
            let (x, y) = (h.x + x_mod, h.y + y_mod);
            let taken = self
                .bt()
                .enemy_units
                .iter()
                .enumerate()
                .any(|(n, u)| n != i && u.alive && u.y == y && (u.x - x).abs() <= 2);
            if !taken {
                return (x, y);
            }
        }
        (h.x, h.y)
    }

    /// `DoAction`: the unit's action for the turn.
    pub fn unit_do_action(&mut self, i: usize, ambush: bool) {
        if self.u(i).pinned {
            let text = if self.unit_morale_test(i, 0, false) {
                let u = self.um(i);
                u.pinned = false;
                if u.morale > 2 {
                    u.morale -= 1;
                }
                " recovers from being Pinned."
            } else {
                " remains Pinned and does nothing."
            };
            let text = format!("{}{text}", self.unit_desc(i, false));
            self.unit_label(i, &text);
            return;
        } else if self.u(i).stunned {
            let text = if self.unit_morale_test(i, 0, false) {
                let u = self.um(i);
                u.stunned = false;
                if u.morale > 2 {
                    u.morale -= 1;
                }
                " recovers from being Stunned."
            } else {
                " remains Stunned and does nothing."
            };
            let text = format!("{}{text}", self.unit_desc(i, false));
            self.unit_label(i, &text);
            return;
        }

        let scen = self.campaign.scen_type.clone();
        let class = self.u(i).unit_class.clone();
        let ranges: [i32; 7] = match class.as_str() {
            "TANK" | "SPG" => match scen.as_str() {
                "Advance" => [10, 30, 40, 60, 65, 80, 85],
                "Battle" => [10, 20, 25, 35, 40, 85, 90],
                _ => [10, 50, 60, 70, 75, 95, 100],
            },
            "APC" => {
                if self.u(i).full_apc && self.uhex(i).rng <= 1 && self.roll_1d6() <= 4 {
                    self.um(i).moving = false;
                    self.unit_dismount_infantry(i, false);
                    return;
                }
                match scen.as_str() {
                    "Advance" => [10, 20, 30, 40, 60, 65, 80],
                    "Battle" => [10, 15, 20, 25, 35, 40, 85],
                    _ => [10, 40, 50, 60, 70, 75, 80],
                }
            }
            "TRUCK" => [30, 40, 75, 110, 0, 0, 0],
            "AC" => {
                if self.u(i).spotting_player {
                    if self.roll_1d6() > 1 {
                        let text = format!("{} continues spotting your tank.", self.unit_desc(i, false));
                        self.unit_label(i, &text);
                        self.um(i).moving = false;
                        return;
                    }
                    let text = format!("{} lost sight of your tank!", self.unit_desc(i, false));
                    self.unit_label(i, &text);
                    let u = self.um(i);
                    u.spotting_player = false;
                    u.moving = false;
                    return;
                }
                [15, 25, 50, 60, 75, 85, 100]
            }
            // the original has no AT gun table for counterattacks, where
            // AT guns do not appear; the advance table is a safe stand-in
            "AT_GUN" => match scen.as_str() {
                "Battle" => [30, 0, 0, 0, 0, 80, 90],
                _ => [30, 0, 0, 0, 0, 65, 70],
            },
            _ => {
                if scen == "Advance" || scen == "Battle" {
                    [10, 20, 40, 60, 95, 0, 0]
                } else {
                    [10, 40, 60, 70, 95, 0, 100]
                }
            }
        };

        for _ in 0..300 {
            let mut result = self.roll_1d100();
            if ambush {
                result += 10;
                if self.u(i).is_class(&["LW", "MG", "AT_GUN"]) {
                    result += 10;
                }
            }
            if result <= ranges[0] {
                let text = format!("{} doesn't appear to do anything.", self.unit_desc(i, false));
                self.unit_label(i, &text);
                self.um(i).moving = false;
                return;
            } else if result <= ranges[1] {
                if self.unit_dist_move(i, -1) {
                    let text = format!("{} moves towards you.", self.unit_desc(i, false));
                    self.unit_label(i, &text);
                    return;
                }
            } else if result <= ranges[2] {
                if self.unit_lateral_move(i) {
                    let text = format!("{} shifts its position.", self.unit_desc(i, false));
                    self.unit_label(i, &text);
                    return;
                }
            } else if result <= ranges[3] {
                if self.unit_dist_move(i, 1) {
                    let text = format!("{} moves further away.", self.unit_desc(i, false));
                    self.unit_label(i, &text);
                    return;
                }
            } else if result <= ranges[4] {
                if self.unit_attack_infantry(i) {
                    return;
                }
            } else if result <= ranges[5] {
                if self.u(i).shot_at {
                    if self.u(i).facing == "Front" {
                        if self.unit_attack_player(i) {
                            return;
                        }
                    } else if self.unit_dist_move(i, 1) {
                        return;
                    }
                }
                if self.unit_attack_tank(i) {
                    return;
                }
            } else if result <= ranges[6] {
                if self.unit_attack_player(i) {
                    return;
                }
            } else {
                if self.tank.stats.str("vehicle_type") == "Sherman VC" && self.roll_1d6() <= 2 && self.unit_attack_player(i) {
                    return;
                }
                if self.tank.lead_tank {
                    if self.unit_attack_player(i) {
                        return;
                    }
                } else if self.unit_attack_tank(i) {
                    return;
                }
            }
            if !self.u(i).alive || self.bt().result != "Undetermined" {
                return;
            }
        }
    }

    /// `ActivateEnemies`: the encounter's first enemy units.
    pub fn activate_enemies(&mut self) {
        let res_level = if self.campaign.scen_type == "Counterattack" || self.bt().counterattack {
            Some(self.campaign.scen_res.clone())
        } else {
            self.campaign.player_node().resistance.clone()
        };
        let n = match res_level.as_deref() {
            Some("Light") => 2,
            Some("Medium") => 3,
            _ => 4,
        };
        for _ in 0..n {
            self.spawn_enemy(None, None);
            self.update_map_overlay(false);
            self.render_encounter(false);
            self.wait(300);
        }
    }

    /// `SpawnEnemy`: a new enemy unit, its class and hex rolled unless
    /// given. Returns its index.
    pub fn spawn_enemy(&mut self, unit_class: Option<&str>, map_hex: Option<usize>) -> Option<usize> {
        let morale = self.roll_morale();
        let mut new_unit = EnemyUnit::new(morale);

        let unit_class = match unit_class {
            Some(c) => c.to_string(),
            None => {
                let column = if self.bt().counterattack {
                    2
                } else {
                    match self.campaign.scen_type.as_str() {
                        "Advance" => 0,
                        "Battle" => 1,
                        _ => 2,
                    }
                };
                // the original's date modifiers never take effect (they
                // change a copy); kept, see CLAUDE.md
                let mut activation_list = self.campaign.mission_activations[column].clone();
                let total: i32 = activation_list.iter().map(|(_, v)| v).sum();
                self.shuffle(&mut activation_list);
                let mut roll = self.rng.int(0, total);
                let mut found = String::new();
                for (k, v) in &activation_list {
                    if *v == 0 {
                        continue;
                    }
                    if roll <= *v {
                        found = k.clone();
                        break;
                    }
                    roll -= v;
                }
                found
            }
        };
        if unit_class.is_empty() {
            log::error!("could not generate a random unit class");
            return None;
        }

        let b = self.bt();
        let mut unit_type = match unit_class.as_str() {
            "TANK" => b.tank_type.clone().unwrap_or_default(),
            "SPG" => b.spg_type.clone().unwrap_or_default(),
            "AT_GUN" => b.at_gun_type.clone().unwrap_or_default(),
            "MG" => "MG Team".into(),
            "LW" => "Light Weapons Infantry".into(),
            _ => String::new(),
        };
        if unit_type.is_empty() {
            let mut result = self.rng.int(1, 1000);
            if let Some((_, list)) = self.campaign.class_activations.iter().find(|(c, _)| *c == unit_class) {
                let mut type_list = list.clone();
                self.shuffle(&mut type_list);
                for (k, v) in &type_list {
                    if *v == 0 {
                        continue;
                    }
                    if result <= *v {
                        unit_type = k.clone();
                        break;
                    }
                    result -= v;
                }
            }
        }
        if unit_type.is_empty() {
            log::error!("could not generate a random unit type for class: {unit_class}");
            return None;
        }

        if ["PzKw V G", "PzKw VI E", "PzKw VI B"].contains(&unit_type.as_str()) && new_unit.morale < 10 {
            new_unit.morale += 1;
        }
        if unit_class == "AT_GUN" {
            new_unit.stats.set("main_gun", defs::Stat::Str(unit_type.clone()));
        }
        let b = self.btm();
        match unit_class.as_str() {
            "TANK" if b.tank_type.is_none() => b.tank_type = Some(unit_type.clone()),
            "SPG" if b.spg_type.is_none() => b.spg_type = Some(unit_type.clone()),
            "AT_GUN" if b.at_gun_type.is_none() => b.at_gun_type = Some(unit_type.clone()),
            _ => {}
        }
        new_unit.unit_type = unit_type.clone();
        new_unit.unit_class = unit_class.clone();
        if ["TANK", "SPG", "APC", "AC", "TRUCK"].contains(&unit_class.as_str()) {
            new_unit.stats = self.vehicle_stats(&unit_type);
        }
        if unit_class == "LW" {
            let mut roll = self.roll_1d6();
            if self.campaign.current_date[0] == 1945 {
                roll -= 1;
            }
            if roll <= 3 {
                new_unit.pf = true;
            }
        } else if unit_class == "APC" {
            if self.roll_1d6() <= 4 {
                new_unit.full_apc = true;
            }
        }

        new_unit.map_hex = match map_hex {
            Some(h) => h,
            None => {
                let (_, _, mut roll) = self.roll_2d6();
                if self.campaign.scen_type == "Counterattack" {
                    roll += 1;
                }
                let sector = if roll <= 6 {
                    4
                } else if roll <= 9 {
                    *self.rng.choice(&[3, 5])
                } else if roll <= 11 {
                    *self.rng.choice(&[2, 0])
                } else {
                    1
                };
                let mut result = self.roll_1d10();
                match self.campaign.player_node().node_type.as_str() {
                    "C" => result -= 3,
                    "D" => result -= 2,
                    "F" => result -= 5,
                    _ => {}
                }
                if self.tank.stats.str("vehicle_type") == "Sherman VC" {
                    result += 3;
                }
                // the original's AT gun row tests for 'AT', never matching
                // 'AT_GUN', so AT guns use the last row; kept
                let rng = match unit_class.as_str() {
                    "LW" => if result <= 6 { 0 } else { 1 },
                    "MG" => if result <= 3 { 0 } else if result <= 8 { 1 } else { 2 },
                    "SPG" => if result <= 2 { 0 } else if result <= 6 { 1 } else { 2 },
                    _ => if result <= 3 { 0 } else if result <= 7 { 1 } else { 2 },
                };
                let b = self.bt();
                let spawn_hexes: Vec<usize> =
                    (0..b.maphexes.len()).filter(|&n| b.maphexes[n].rng == rng && b.maphexes[n].sector == sector).collect();
                *self.rng.choice(&spawn_hexes)
            }
        };

        self.btm().enemy_units.push(new_unit);
        let i = self.bt().enemy_units.len() - 1;
        let (x, y) = self.unit_char_location(i);
        let u = self.um(i);
        u.x = x;
        u.y = y;
        self.unit_set_facing(i);
        self.unit_set_terrain(i);

        let h = self.uhex(i).clone();
        let clock = match h.sector {
            0 => "four o'clock",
            1 => "six o'clock",
            2 => "eight o'clock",
            3 => "ten o'clock",
            4 => "twelve o'clock",
            _ => "two o'clock",
        };
        let range = match h.rng {
            0 => "close range!",
            1 => "medium range.",
            _ => "long range.",
        };
        let text = format!("{} reported at {clock}, {range}", self.unit_desc(i, true));
        self.play_sound("radio");
        self.unit_label(i, &text);
        Some(i)
    }

    /// `random.shuffle`
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for n in (1..items.len()).rev() {
            let k = self.rng.int(0, n as i32) as usize;
            items.swap(n, k);
        }
    }
}
