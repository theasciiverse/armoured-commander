//! `Crewman`'s methods and `SpawnCrewMember`, as `Game` methods taking the
//! crewman's index in `tank.crew`.

use crate::color;
use crate::console::Align;
use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::model::*;

impl Game {
    /// `ResolveKIA`
    pub fn crew_resolve_kia(&mut self, i: usize) {
        let c = self.crewm(i);
        c.no_bail = true;
        c.alive = false;
        c.unconscious = false;
        c.stunned = false;
        c.light_wound = false;
        c.serious_wound = false;
        c.v_serious_wound = false;
        self.crew_set_spot_ability(i);
        self.crew_reset_order(i, false);
        self.crew_add_headstone(i);
    }

    /// `GenerateReport`: the crewman's record, for the journal.
    pub fn crew_generate_report(&self, i: usize) -> Vec<String> {
        let c = self.crew(i);
        let mut lines: Vec<String> = vec!["".into(), "****".into(), "Crewman Report".into()];
        lines.push(self.crew_rank(i, false));
        lines.push(c.name.clone());
        if !c.nickname.is_empty() {
            lines.push(c.nickname.clone());
        }
        lines.push(format!(" {}", c.hometown));
        lines.push(String::new());
        if !c.alive {
            lines.push(format!("KIA, {}", self.campaign.get_date(None)));
            lines.push(String::new());
        } else if c.v_serious_wound {
            lines.push(format!("Sent Home, {}", self.campaign.get_date(None)));
            lines.push(String::new());
        }
        lines.push("Skills:".into());
        if c.skills.is_empty() {
            lines.push("None".into());
        } else {
            for s in &c.skills {
                let mut string = format!(" {}", s.name);
                if s.level < 100 {
                    string.push_str(&format!(": {}%", s.level));
                }
                lines.push(string);
            }
        }
        lines.push(String::new());
        lines.push("Decorations:".into());
        if c.decorations.is_empty() {
            lines.push("None".into());
        } else {
            lines.extend(c.decorations.iter().cloned());
        }
        lines
    }

    /// `AddHeadStone`
    pub fn crew_add_headstone(&mut self, i: usize) {
        let c = self.crew(i);
        let decoration = c.decorations.last().cloned().unwrap_or_default();
        let entry = [self.crew_rank(i, false), c.name.clone(), c.hometown.clone(), self.campaign.get_date(None), decoration];
        crate::bones::add_headstone(entry);
    }

    /// `AwardDecoration`: add it and show the citation.
    pub fn crew_award_decoration(&mut self, i: usize, dec_name: &str) {
        let name = self.crew(i).name.clone();
        self.write_journal(&format!("{name} awarded new decoration: {dec_name}"));
        self.crewm(i).decorations.push(dec_name.to_string());

        self.darken_screen();
        let rank = self.crew_rank(i, false);
        let (nickname, alive) = (self.crew(i).nickname.clone(), self.crew(i).alive);
        let m = &mut self.menu_con;
        m.bg = color::BLACK;
        m.fg = color::WHITE;
        m.clear();
        m.align = Align::Center;
        m.frame(0, 0, MENU_CON_WIDTH - 1, MENU_CON_HEIGHT - 1, false, false);
        m.fg = MENU_TITLE_COLOR;
        m.print(MENU_CON_XM, 2, "Decoration Award");
        m.fg = color::WHITE;
        m.print(MENU_CON_XM, 5, &format!("{rank} {name}"));
        if !nickname.is_empty() {
            m.print(MENU_CON_XM, 6, &format!("\"{nickname}\""));
        }
        self.decoration_step();

        let text = format!("has been {}awarded the", if alive { "" } else { "posthumously " });
        self.menu_con.print(MENU_CON_XM, 7, &text);
        self.decoration_step();

        self.menu_con.print(MENU_CON_XM, 10, dec_name);
        self.decoration_step();

        let text = if dec_name == "Purple Heart" {
            "for wounds received in action".to_string()
        } else {
            self.campaign.decorations.iter().find(|d| d.0 == dec_name).map(|d| d.1.clone()).unwrap_or_default()
        };
        self.menu_con.print(MENU_CON_XM, 13, &text);
        self.decoration_step();

        let text = format!("this {}", self.campaign.get_date(None));
        self.menu_con.print(MENU_CON_XM, 15, &text);
        self.menu_con.print(MENU_CON_XM, MENU_CON_HEIGHT - 3, &format!("[{}] to continue", hl("Enter")));
        self.decoration_step();
        self.menu_con.align = Align::Left;
        self.wait_for_enter();
    }

    fn decoration_step(&mut self) {
        self.blit_menu();
        self.flush();
        self.wait(400);
    }

    /// `RecoveryRoll`
    pub fn crew_recovery_roll(&mut self, i: usize) {
        if self.crew(i).unconscious {
            let (_, _, mut roll) = self.roll_2d6();
            if self.crew_skill_check(i, "True Grit") {
                roll -= 1;
            }
            if roll <= 8 {
                let c = self.crewm(i);
                c.unconscious = false;
                c.stunned = true;
                let text = format!("{} regains consciousness but remains Stunned.", c.name);
                self.message(&text);
            }
        } else if self.crew(i).stunned {
            let (_, _, mut roll) = self.roll_2d6();
            if self.crew_skill_check(i, "True Grit") {
                roll -= 1;
            }
            if roll <= 8 {
                self.crewm(i).stunned = false;
                let text = format!("{} recovers from being Stunned.", self.crew(i).name);
                self.message(&text);
            }
        }
    }

    /// `SetLevel`
    pub fn crew_set_level(&mut self, i: usize, new_level: i32) {
        let c = self.crewm(i);
        if new_level == c.level {
            return;
        }
        c.skill_pts += new_level - c.level;
        c.level = new_level;
        c.exp = Game::get_exp_req(new_level);
    }

    /// `BuildOrdersList`: the orders this crewman can be given now.
    pub fn crew_build_orders_list(&mut self, i: usize, no_reset: bool) {
        let t = &self.tank;
        let c = &t.crew[i];
        let mut list = Vec::new();
        for order in &defs::defs().crew_orders {
            let name = order.name.as_str();
            if !order.position_list.contains(&c.position) {
                continue;
            }
            if name == "Fire Smoke Mortar" && !t.stats.has("smoke_mortar") {
                continue;
            }
            if (t.immobilized || t.bogged_down)
                && order.position_list.iter().any(|p| p == "Driver")
                && !["Stop", "Attempt Unbog", "Abandon Tank"].contains(&name)
            {
                continue;
            }
            if name == "Attempt Unbog" && !t.bogged_down {
                continue;
            }
            if name == "Throw Smoke Grenade" && (c.hatch != "Open" || t.smoke_grenades == 0) {
                continue;
            }
            if name == "Reload" && t.ammo_load == "None" {
                continue;
            }
            if name == "Fire AA MG" || name == "Repair AA MG" {
                if !t.stats.has("aa_mg") {
                    continue;
                }
                if c.position == "Loader" && t.stats.str("loader_hatch") != "Split" {
                    continue;
                }
                if c.hatch != "Open" {
                    continue;
                }
            }
            if name == "Fire AA MG" && (t.has_damage("AA MG Malfunction") || t.has_damage("AA MG Broken")) {
                continue;
            }
            if name == "Repair AA MG" && !t.has_damage("AA MG Malfunction") {
                continue;
            }
            if name == "Fire Co-Axial MG"
                && (!t.stats.has("co_ax_mg") || t.has_damage("Co-ax MG Malfunction") || t.has_damage("Co-ax MG Broken"))
            {
                continue;
            }
            if name == "Repair Co-ax MG" && (!t.stats.has("co_ax_mg") || !t.has_damage("Co-ax MG Malfunction")) {
                continue;
            }
            if name == "Fire Bow MG"
                && (!t.stats.has("bow_mg")
                    || t.hull_down
                    || t.has_damage("Bow MG Malfunction")
                    || t.has_damage("Bow MG Broken"))
            {
                continue;
            }
            if name == "Repair Bow MG" && (!t.stats.has("bow_mg") || !t.has_damage("Bow MG Malfunction")) {
                continue;
            }
            if ["Fire Main Gun", "Direct Main Gun Fire", "Change Gun Load"].contains(&name)
                && (t.has_damage("Main Gun Malfunction") || t.has_damage("Main Gun Broken") || t.has_damage("Gun Sight Broken"))
            {
                continue;
            }
            if (t.has_damage("Turret Traverse Malfunction") || t.has_damage("Turret Traverse Broken")) && name == "Rotate Turret" {
                continue;
            }
            if (t.has_damage("Intercom Malfunction") || t.has_damage("Intercom Broken"))
                && (name == "Direct Movement" || name == "Direct Bow MG Fire")
            {
                continue;
            }
            match name {
                "Repair Main Gun" if !t.has_damage("Main Gun Malfunction") => continue,
                "Repair Turret Traverse" if !t.has_damage("Turret Traverse Malfunction") => continue,
                "Repair Radio" if !t.has_damage("Radio Malfunction") => continue,
                "Repair Intercom" if !t.has_damage("Intercom Malfunction") => continue,
                _ => {}
            }
            if name == "Abandon Tank" {
                let qualify =
                    t.immobilized || t.swiss_cheese || t.crew.iter().any(|cm| cm.v_serious_wound || !cm.alive);
                if !qualify {
                    continue;
                }
            }
            list.push(order.clone());
        }
        self.crewm(i).orders_list = list;

        if !no_reset {
            self.crew_reset_order(i, false);
            return;
        }
        if !self.crew(i).current_order_ok() {
            self.crew_reset_order(i, false);
        }
    }

    /// `SkillCheck`: if the crewman has the skill, roll to use it.
    pub fn crew_skill_check(&mut self, i: usize, skill_name: &str) -> bool {
        let c = self.crew(i);
        if c.no_actions() {
            return false;
        }
        let Some(level) = c.skills.iter().find(|s| s.name == skill_name).map(|s| s.level) else {
            return false;
        };
        let mut roll = self.roll_1d100();
        if let Some(b) = &self.battle {
            if b.battle_leadership {
                roll -= 5;
            }
        }
        if roll <= level && roll <= 95 {
            if level < 100 {
                let text = format!("{} activates {} skill!", self.crew(i).name, skill_name);
                self.message_col(&text, SKILL_ACTIVATE_COLOR);
            }
            return true;
        }
        false
    }

    /// Skill check by position; false if nobody is in it.
    pub fn pos_skill_check(&mut self, position: &str, skill_name: &str) -> bool {
        match self.crew_pos(position) {
            Some(i) => self.crew_skill_check(i, skill_name),
            None => false,
        }
    }

    /// `GetRank`
    pub fn crew_rank(&self, i: usize, short: bool) -> String {
        let r = &self.campaign.ranks[self.crew(i).rank_level.min(self.campaign.ranks.len().saturating_sub(1))];
        if short { r.0.clone() } else { r.1.clone() }
    }

    /// `GenerateName`. The original also meant to skip names already in
    /// the crew, but its `continue` only left the inner loop; that is
    /// done properly here.
    pub fn generate_name(&mut self) -> String {
        let d = defs::defs();
        loop {
            let name = format!("{} {}", self.rng.choice(&d.first_names), self.rng.choice(&d.last_names));
            if name.len() > NAME_MAX_LEN {
                continue;
            }
            if self.tank.crew.iter().any(|c| c.name == name) {
                continue;
            }
            return name;
        }
    }

    /// `CheckHatch`: after a change of tank, the loader's hatch follows
    /// whether the new one has a loader's hatch.
    pub fn crew_check_hatch(&mut self, i: usize) {
        if self.crew(i).position != "Loader" {
            return;
        }
        let none = self.tank.stats.str("loader_hatch") == "None";
        let c = self.crewm(i);
        if none {
            c.hatch = "None".into();
        } else if c.hatch == "None" {
            c.hatch = "Shut".into();
        }
    }

    /// `SetSpotAbility`
    pub fn crew_set_spot_ability(&mut self, i: usize) {
        let t = &self.tank;
        let c = &t.crew[i];
        let spot: &str = 'spot: {
            if c.no_actions() {
                break 'spot "None";
            }
            if let Some(order) = defs::defs().crew_orders.iter().find(|o| o.name == c.order) {
                if !order.spot {
                    break 'spot "None";
                }
            }
            match c.position.as_str() {
                "Commander" => {
                    if t.stats.has("vision_cupola") || c.hatch == "Open" {
                        "All"
                    } else if t.has_damage("Commander Periscope Broken") {
                        "None"
                    } else {
                        "Any One Sector"
                    }
                }
                "Gunner" => {
                    if t.has_damage("Gunner Periscope Broken") {
                        "None"
                    } else {
                        "Turret Front"
                    }
                }
                "Loader" => {
                    if t.fired_main_gun {
                        "None"
                    } else {
                        let s = if t.stats.str("loader_hatch") != "None" && c.hatch == "Open" {
                            "All"
                        } else {
                            "Any One Sector"
                        };
                        if s == "Any One Sector" && t.has_damage("Loader Periscope Broken") { "None" } else { s }
                    }
                }
                "Driver" | "Asst. Driver" => {
                    if c.hatch == "Open" {
                        "All Except Rear"
                    } else if c.position == "Driver" && t.has_damage("Driver Periscope Broken") {
                        "None"
                    } else if c.position == "Asst. Driver" && t.has_damage("Asst. Driver Periscope Broken") {
                        "None"
                    } else {
                        "Tank Front"
                    }
                }
                _ => "None",
            }
        };
        self.crewm(i).spot = spot.to_string();
    }

    /// `ResetOrder`: the default order for the loader and driver (or
    /// everyone with `reset_all`), none for anyone out of action.
    pub fn crew_reset_order(&mut self, i: usize, reset_all: bool) {
        let c = self.crewm(i);
        if c.no_actions() {
            c.order = "None".into();
        } else if reset_all || c.position == "Loader" || c.position == "Driver" {
            c.order = c.default_order.clone();
        }
        if !c.current_order_ok() {
            c.order = "None".into();
        }
    }

    /// `StunCheck`
    pub fn crew_stun_check(&mut self, i: usize, base_score: i32) -> bool {
        if self.crew(i).no_actions() {
            return false;
        }
        let (_, _, mut roll) = self.roll_2d6();
        if roll != 12 {
            if self.crew_skill_check(i, "True Grit") {
                roll -= 1;
            }
            if roll <= base_score {
                return false;
            }
        }
        self.crewm(i).stunned = true;
        true
    }

    /// `TakeWound`: returns a description of the wound, or None.
    pub fn crew_take_wound(
        &mut self,
        i: usize,
        hit_location: Option<&str>,
        sector: Option<i32>,
        collateral: bool,
        minefield: bool,
    ) -> Option<String> {
        if !self.crew(i).alive {
            return None;
        }
        if (self.crew(i).stunned || self.crew(i).unconscious) && collateral {
            return None;
        }
        let name = self.crew(i).name.clone();
        let (_, _, mut roll) = self.roll_2d6();

        if roll == 12 {
            if self.crew_skill_check(i, "Pocket Bible") {
                self.write_journal(&format!("{name} was saved by Pocket Bible."));
                return Some("Saved by Pocket Bible".into());
            }
            if collateral && self.crew_skill_check(i, "Lightning Reflexes") {
                self.write_journal(&format!("{name} was saved by Lightning Reflexes."));
                return Some("Saved by Lightning Reflexes".into());
            }
            self.crew_resolve_kia(i);
            self.update_tank_con();
            return Some("Killed!".into());
        }

        if let (Some(loc), Some(sector)) = (hit_location, sector) {
            let (mut d, mut a, mut g, mut l, mut c) = (0, 0, 0, 0, 0);
            // the original tested for 'turret' in lower case, which never
            // matched its 'Turret', so every hit counted as a hull hit;
            // fixed (David, 2026-10-19)
            if loc == "Turret" {
                d -= 2;
                a -= 2;
                if sector == 5 || sector == 0 {
                    g += 1;
                    l -= 1;
                } else if sector == 2 || sector == 3 {
                    g -= 1;
                    l += 1;
                } else if sector == 1 {
                    c += 1;
                }
            } else if sector == 5 {
                d -= 1;
                l -= 1;
                a += 1;
                g += 1;
            } else if [0, 1, 2].contains(&sector) {
                d -= 3;
                a -= 3;
                g -= 2;
                l -= 2;
                c -= 2;
            } else if sector == 3 {
                d += 1;
                l += 1;
                a -= 1;
                g -= 1;
            }
            roll += match self.crew(i).position.as_str() {
                "Commander" => c,
                "Gunner" => g,
                "Loader" => l,
                "Driver" => d,
                "Asst. Driver" => a,
                _ => 0,
            };
        }

        let cm = self.crew(i);
        if cm.v_serious_wound {
            roll += 3;
        } else if cm.serious_wound {
            roll += 2;
        } else if cm.light_wound {
            roll += 1;
        }
        if minefield {
            roll -= 2;
        }
        if collateral {
            roll -= 3;
        }
        if cm.order == "Fire AA MG" {
            if collateral {
                roll += 4;
            } else if hit_location.is_some() && sector.is_some() {
                roll -= 2;
            }
        }
        if self.crew_skill_check(i, "True Grit") {
            roll -= 1;
        }

        if roll <= 6 {
            return None;
        }
        if collateral && self.crew_skill_check(i, "Lightning Reflexes") {
            return Some("Saved by Lightning Reflexes".into());
        }

        let text = match roll {
            7 => {
                self.crewm(i).light_wound = true;
                if self.crew_stun_check(i, 10) { "Light Wound, Stunned" } else { "Light Wound" }
            }
            8 => {
                self.crewm(i).light_wound = true;
                if self.crew_stun_check(i, 7) { "Light Wound, Stunned" } else { "Light Wound" }
            }
            9 => {
                self.crewm(i).serious_wound = true;
                if self.crew_stun_check(i, 5) { "Serious Wound, Stunned" } else { "Serious Wound" }
            }
            10 => {
                let c = self.crewm(i);
                c.serious_wound = true;
                c.stunned = true;
                "Serious Wound, Stunned"
            }
            11 => {
                let c = self.crewm(i);
                c.v_serious_wound = true;
                c.unconscious = true;
                c.stunned = false;
                "Very Serious Wound, Unconscious"
            }
            _ => {
                if self.crew_skill_check(i, "Pocket Bible") {
                    self.write_journal(&format!("{name} was saved by Pocket Bible."));
                    return Some("Saved by Pocket Bible".into());
                }
                self.crew_resolve_kia(i);
                "Killed!"
            }
        };

        if self.crew(i).no_actions() {
            self.crew_build_orders_list(i, false);
            self.crew_set_spot_ability(i);
            self.crew_reset_order(i, false);
        }
        self.update_tank_con();
        self.write_journal(&format!("{name} was wounded, result: {text}"));
        Some(text.to_string())
    }

    /// `BailOut`
    pub fn crew_bail_out(&mut self, i: usize) -> &'static str {
        if self.crew(i).unconscious {
            return "Cannot bail out";
        }
        if self.crew(i).order == "Fire AA MG" {
            self.crewm(i).bailed_out = true;
            return "Passed";
        }
        let (_, _, mut roll) = self.roll_2d6();
        let mut bail_mod = 0;
        if self.crew(i).hatch == "None" {
            bail_mod += 1;
        }
        if self.crew(i).stunned {
            bail_mod += 1;
        }
        if self.crew_skill_check(i, "Gymnast") {
            bail_mod -= 2;
        }
        roll += bail_mod;
        if roll <= 10 {
            self.crewm(i).bailed_out = true;
            "Passed"
        } else {
            "Failed"
        }
    }

    /// `SpawnCrewMember`: a new crewman for the player's tank.
    pub fn spawn_crew_member(&mut self, name: Option<&str>, position: &str, rank_level: usize) -> Option<usize> {
        if position == "Asst. Driver" && self.tank.stats.has("no_asst_driver") {
            return None;
        }
        let mut c = Crewman::new();
        c.name = match name {
            Some(n) => n.to_string(),
            None => self.generate_name(),
        };
        c.position = position.to_string();
        c.rank_level = rank_level;
        let d = defs::defs();
        if self.campaign.player_nation == "USA" {
            c.hometown = self.rng.choice(&d.usa_hometowns).clone();
        } else if self.campaign.player_nation == "CAN" {
            c.hometown = self.rng.choice(&d.can_hometowns).clone();
        }
        let (order, hatch) = match position {
            "Commander" => ("None", "Open"),
            "Gunner" => ("None", "None"),
            "Loader" => ("Reload", if self.tank.stats.str("loader_hatch") != "None" { "Open" } else { "None" }),
            "Driver" => ("Stop", "Open"),
            _ => ("None", "Open"),
        };
        c.default_order = order.into();
        c.hatch = hatch.into();
        c.order = c.default_order.clone();
        self.tank.crew.push(c);
        let i = self.tank.crew.len() - 1;
        self.crew_set_spot_ability(i);
        let text = format!("{} assigned as: {} {}", position, self.crew_rank(i, true), self.crew(i).name);
        self.write_journal(&text);
        Some(i)
    }

    /// `SetVehicleStats` for the player's tank: its stats, HVSS rolled for
    /// late 1944 on, and the ammo types its gun fires.
    pub fn set_tank_stats(&mut self) {
        let stats = self.vehicle_stats(&self.tank.unit_type.clone());
        self.tank.stats = stats;
        let t = &mut self.tank;
        t.general_ammo.set("HE", 0);
        t.general_ammo.set("AP", 0);
        t.rr_ammo.set("HE", 0);
        t.rr_ammo.set("AP", 0);
        match t.stats.str("main_gun") {
            "75" => {
                for a in ["WP", "HCBI"] {
                    t.general_ammo.set(a, 0);
                    t.rr_ammo.set(a, 0);
                }
            }
            "76L" => {
                t.general_ammo.set("HVAP", 0);
                t.rr_ammo.set("HVAP", 0);
            }
            "76LL" => {
                t.general_ammo.set("APDS", 0);
                t.rr_ammo.set("APDS", 0);
            }
            _ => {}
        }
    }

    /// `SetVehicleStats`: a unit's stats from its vehicle type.
    pub fn vehicle_stats(&mut self, unit_type: &str) -> defs::Stats {
        let Some(v) = defs::vehicle_type(unit_type) else {
            log::error!("vehicle type not found: {unit_type}");
            return defs::Stats::default();
        };
        let mut stats = defs::Stats::default();
        for (k, value) in v.stats.iter() {
            if k == "HVSS" {
                let (year, month) = if self.campaign.current_date == [0, 0, 0] {
                    let d = &self.campaign.days[0];
                    (d.year, d.month)
                } else {
                    (self.campaign.current_date[0], self.campaign.current_date[1])
                };
                if year >= 1945 || (year == 1944 && month >= 11) {
                    let chance = if let defs::Stat::Int(n) = value { *n } else { 0 };
                    if self.rng.int(1, 10) <= chance {
                        stats.set("HVSS", defs::Stat::Bool(true));
                    }
                }
            } else if *value == defs::Stat::Str(String::new()) {
                stats.set(k, defs::Stat::Bool(true));
            } else {
                stats.set(k, value.clone());
            }
        }
        stats
    }
}
