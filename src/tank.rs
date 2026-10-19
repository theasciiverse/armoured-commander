//! `PlayerTank`'s methods that reach beyond the tank itself.

use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::model::*;

impl Game {
    /// `ResetAfterEncounter`
    pub fn tank_reset_after_encounter(&mut self) {
        let t = &mut self.tank;
        t.reset();
        t.turret_facing = 4;
        t.old_t_facing = 4;
        t.bogged_down = false;
        t.hull_down = false;
        t.moving = false;
        t.fired_main_gun = false;
        for i in 0..self.tank.crew.len() {
            self.crew_reset_order(i, true);
            let c = self.crewm(i);
            c.bailed_out = false;
            c.bail_mod = 0;
        }
    }

    /// `TakeDamage`: a given minor damage, or a random one.
    pub fn tank_take_damage(&mut self, damage_type: Option<&str>, light_weapons: bool, large_gun: bool) {
        let damage_type = match damage_type {
            Some(d) => d.to_string(),
            None => {
                let (_, _, mut roll) = self.roll_2d6();
                if light_weapons {
                    roll += 5;
                } else if large_gun {
                    roll -= 2;
                }
                let d6roll = self.roll_1d6();
                if roll <= 2 {
                    if d6roll <= 3 { "Gun Sight Broken" } else { "Engine Knocked Out" }.to_string()
                } else if roll == 3 {
                    if d6roll == 1 { "Main Gun Broken" } else { "Main Gun Malfunction" }.to_string()
                } else if roll == 4 {
                    if d6roll <= 2 { "Turret Traverse Broken" } else { "Turret Traverse Malfunction" }.to_string()
                } else if roll == 5 {
                    if d6roll <= 3 { "Radio Broken" } else { "Radio Malfunction" }.to_string()
                } else if roll == 6 {
                    if d6roll <= 1 { "Intercom Broken" } else { "Intercom Malfunction" }.to_string()
                } else if roll <= 8 {
                    let base = if d6roll <= 2 {
                        if !self.tank.stats.has("aa_mg") {
                            return;
                        }
                        "AA MG"
                    } else if d6roll <= 4 {
                        if !self.tank.stats.has("co_ax_mg") {
                            return;
                        }
                        "Co-ax MG"
                    } else {
                        if !self.tank.stats.has("bow_mg") {
                            return;
                        }
                        "Bow MG"
                    };
                    let suffix = if self.roll_1d6() <= 2 { " Broken" } else { " Malfunction" };
                    format!("{base}{suffix}")
                } else if roll <= 10 {
                    let pos = match d6roll {
                        1 => "Commander",
                        2 => "Gunner",
                        3 => "Loader",
                        4 => "Driver",
                        5 => "Asst. Driver",
                        _ => return,
                    };
                    format!("{pos} Periscope Broken")
                } else {
                    return;
                }
            }
        };

        if self.tank.has_damage(&damage_type) {
            return;
        }
        if let Some(d) = defs::defs().damage_type(&damage_type) {
            if self.tank.has_damage(&d.break_result) {
                return;
            }
        }
        let mal_result = damage_type.replace("Broken", "Malfunction");
        self.tank.damage_list.retain(|d| *d != mal_result);
        self.tank.damage_list.push(damage_type.clone());
        self.popup(&format!("Your tank has been damaged: {damage_type}"));
        if damage_type == "Engine Knocked Out" {
            self.tank.immobilized = true;
        }
        for i in 0..self.tank.crew.len() {
            self.crew_build_orders_list(i, true);
            self.crew_set_spot_ability(i);
        }
        self.update_tank_con();
    }

    /// `ChangeGunLoadMenu`
    pub fn tank_change_gun_load_menu(&mut self) {
        let t = &self.tank;
        let choice_list: Vec<String> = AMMO_TYPES
            .iter()
            .filter(|a| t.general_ammo.has(a) && (t.general_ammo.get(a) > 0 || t.rr_ammo.get(a) > 0))
            .map(|a| a.to_string())
            .collect();
        if choice_list.is_empty() {
            self.popup("No shells remaining, can't change gun load");
            return;
        }
        let Some(choice) = self.get_choice("Select type of shell to load in main gun", &choice_list) else { return };
        let t = &mut self.tank;
        if t.ammo_load != "None" {
            let load = t.ammo_load.clone();
            t.general_ammo.add(&load, 1);
        }
        if t.general_ammo.get(&choice) > 0 {
            t.general_ammo.add(&choice, -1);
        } else {
            t.rr_ammo.add(&choice, -1);
        }
        t.ammo_load = choice;
    }

    /// `ToggleHatch`
    pub fn tank_toggle_hatch(&mut self, crewman: Option<usize>) {
        let Some(i) = crewman else { return };
        let hatch = self.crew(i).hatch.clone();
        if hatch == "None" {
            return;
        }
        if hatch == "Open" {
            self.crewm(i).hatch = "Shut".into();
            self.play_sound("hatch_close");
        } else {
            self.crewm(i).hatch = "Open".into();
            self.play_sound("hatch_open");
        }
        self.crew_set_spot_ability(i);
        self.crew_build_orders_list(i, true);
    }

    /// `SetLeadTank`
    pub fn tank_set_lead_tank(&mut self) {
        let vt = self.tank.stats.str("vehicle_type").to_string();
        if vt == "Sherman VC" {
            return;
        }
        let lead_tank = if !vt.contains("M4A3E2") {
            if self.tank.lead_tank {
                self.tank.lead_tank = false;
                return;
            }
            11
        } else {
            8
        };
        let (_, _, roll) = self.roll_2d6();
        if roll >= lead_tank {
            self.tank.lead_tank = true;
            self.popup("Your tank is the Lead Tank for the day");
            let text = format!("\"{}\" is assigned to be Lead Tank for the day", self.tank.name);
            self.write_journal(&text);
        } else {
            self.tank.lead_tank = false;
        }
    }

    /// `SetDeployment`: hull down, stopped or moving at the start of an
    /// encounter.
    pub fn tank_set_deployment(&mut self) {
        if self.campaign.scen_type == "Counterattack" || self.bt().counterattack {
            self.tank.hull_down = true;
            return;
        }
        let vt = self.tank.stats.str("vehicle_type").to_string();
        let (mut hull_down, stopped) = if vt == "Sherman VC" {
            (7, 9)
        } else if vt.contains("M4A3E2") {
            (3, 5)
        } else {
            (5, 7)
        };
        if self.campaign.player_node().node_type == "F" {
            hull_down += 2;
        }
        if self.pos_skill_check("Driver", "Cautious Driver") {
            hull_down += 2;
        }
        let (_, _, roll) = self.roll_2d6();
        if roll <= hull_down {
            self.tank.hull_down = true;
            self.message("Your tank is Hull Down");
        } else if roll <= stopped {
            self.message("Your tank is Stopped");
        } else {
            self.tank.moving = true;
            self.message("Your tank is Moving");
        }
    }

    /// `Penetrate`: the effects of a penetrating hit on the player's tank.
    pub fn tank_penetrate(&mut self, hit_location: &str, sector: i32, gun_type: &str, critical: bool) {
        let mut modifier = 0;
        if hit_location == "Hull" {
            modifier -= 1;
        }
        let mut large_gun = false;
        if ["88L", "88LL", "Panzerfaust"].contains(&gun_type) {
            modifier -= 2;
            large_gun = true;
        }
        let (_, _, mut roll) = self.roll_2d6();
        if critical {
            let (_, _, roll2) = self.roll_2d6();
            if roll2 < roll {
                roll = roll2;
            }
        }
        if [3, 4, 5, 11, 12].contains(&(roll + modifier)) {
            if !self.tank.stats.has("wet_stowage") {
                modifier -= 1;
            }
            let mut total: i32 = AMMO_TYPES.iter().filter(|a| self.tank.general_ammo.has(a)).map(|a| self.tank.general_ammo.get(a)).sum();
            total -= self.tank.stats.int("main_gun_rounds");
            if total > 0 && self.rng.int(1, 100) <= total {
                modifier -= 1;
            }
        }
        roll += modifier;

        if roll <= 2 {
            self.play_sound("tank_knocked_out");
            let text = "Your tank explodes, killing the entire crew!";
            self.popup(text);
            self.write_journal(text);
            self.campaign.add_stat("Tanks Lost", 1);
            self.tank.alive = false;
            self.btm().result = "Tank Lost".into();
            for i in 0..self.tank.crew.len() {
                self.crew_resolve_kia(i);
            }
        } else if roll <= 7 {
            self.play_sound("tank_knocked_out");
            let text = "Your tank has been knocked out, and the crew must bail out.";
            self.popup(text);
            self.write_journal(text);
            self.campaign.add_stat("Tanks Lost", 1);
            self.tank.alive = false;
            self.btm().result = "Tank Lost".into();
            self.resolve_crew_fate(Some(hit_location), Some(sector), gun_type == "Panzerfaust", false);
        } else if roll <= 8 {
            let text = "Fragments of shell and armour ricochet throughout the tank, causing multiple wounds and damage.";
            self.popup(text);
            self.write_journal(text);
            self.tank.swiss_cheese = true;
            for _ in 0..3 {
                self.tank_take_damage(None, false, large_gun);
            }
            for i in 0..self.tank.crew.len() {
                for _ in 0..2 {
                    if let Some(text) = self.crew_take_wound(i, Some(hit_location), Some(sector), false, false) {
                        let text = format!("{} is wounded! Result: {text}", self.crew(i).name);
                        self.popup(&text);
                    }
                }
            }
        } else if roll <= 9 {
            let text = "The shell impact causes spalling in the tank's armour, sending metal fragments bursting into the crew compartment.";
            self.popup(text);
            self.write_journal(text);
            self.tank.swiss_cheese = true;
            for _ in 0..2 {
                self.tank_take_damage(None, false, large_gun);
            }
            for i in 0..self.tank.crew.len() {
                if let Some(text) = self.crew_take_wound(i, Some(hit_location), Some(sector), false, false) {
                    let text = format!("{} is wounded! Result: {text}", self.crew(i).name);
                    self.popup(&text);
                }
            }
        } else if roll <= 10 {
            let text = "The shell does no serious damage, but ignites a fire inside the crew compartment which is quickly extinguished.";
            self.popup(text);
            self.write_journal(text);
            self.tank_take_damage(None, false, large_gun);
            let n = self.tank.crew.len() as i32;
            let i = self.rng.int(0, n - 1) as usize;
            if let Some(text) = self.crew_take_wound(i, None, None, true, false) {
                let text = format!("{} is wounded! Result: {text}", self.crew(i).name);
                self.popup(&text);
            }
        } else {
            let text = "The shell impacts a well-protected area, causing only minor damage.";
            self.popup(text);
            self.write_journal(text);
            self.tank_take_damage(None, false, large_gun);
        }
    }

    /// `AbandonTank`
    pub fn tank_abandon(&mut self) {
        let text = "Your crew abandons your tank and bails out.";
        self.popup(text);
        self.write_journal(text);
        self.campaign.add_stat("Tanks Lost", 1);
        self.tank.alive = false;
        self.btm().result = "Tank Lost".into();
        for i in 0..self.tank.crew.len() {
            self.crew_recovery_roll(i);
        }
        self.resolve_crew_fate(None, None, false, true);
    }

    /// `LWAttack`: small arms against anyone with an open hatch, and a
    /// chance of minor damage.
    pub fn tank_lw_attack(&mut self) {
        let mut hit_result = false;
        for i in 0..self.tank.crew.len() {
            if self.crew(i).hatch == "Open" {
                if let Some(text) = self.crew_take_wound(i, None, None, true, false) {
                    hit_result = true;
                    let text = format!("{} is hit! Result: {text}", self.crew(i).name);
                    self.popup(&text);
                }
            }
        }
        if !hit_result {
            self.show_label(MAP_X0 + map_con_x(), MAP_Y0 + map_con_y(), "No crewmembers affected.", None);
        }
        if self.roll_1d6() == 1 {
            self.tank_take_damage(None, true, false);
        }
    }

    /// `MinefieldAttack`
    pub fn tank_minefield_attack(&mut self) {
        let roll = self.roll_1d10();
        if roll >= 3 {
            self.popup("Luckily your tanks do not trigger any mines.");
            return;
        } else if roll <= 1 {
            self.popup("One friendly tank is knocked out by a mine blast.");
            self.btm().tanks_lost += 1;
            return;
        }
        let text = "Your tank triggers a mine which explodes, disabling it!";
        self.popup(text);
        self.write_journal(text);
        self.tank.moving = false;
        self.tank.immobilized = true;
        if let Some(d) = self.crew_pos("Driver") {
            self.crew_build_orders_list(d, false);
        }
        self.update_tank_con();

        let roll = self.roll_1d10();
        if roll <= 8 {
            self.popup("Luckily, none of your crew is injured by the blast.");
            return;
        }
        let crew = self.crew_pos(if roll == 9 { "Driver" } else { "Asst. Driver" });
        let result = crew.and_then(|c| self.crew_take_wound(c, None, None, false, true));
        let (Some(c), Some(result)) = (crew, result) else {
            self.popup("Luckily, none of your crew is injured by the blast.");
            return;
        };
        let text = format!("{} is hit by the blast! Result: {result}", self.crew(c).name);
        self.popup(&text);
        self.write_journal(&text);
    }

    /// `PlaySound`. Sound is not ported yet; this marks where the original
    /// plays one.
    pub fn play_sound(&mut self, name: &str) {
        if self.campaign.sounds {
            log::trace!("sound: {name}");
        }
    }
}
