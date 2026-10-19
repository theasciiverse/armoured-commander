//! "Encounter Handling" and "Set up and Handle an Encounter" in
//! armcom.py: the encounter menu, the phase loop and its input, and
//! starting an encounter.

use crate::color;
use crate::console::Align;
use crate::defs;
use crate::enemy::firing_sound;
use crate::game::*;
use crate::layout::*;
use crate::model::*;
use crate::ui::Vk;

impl Game {
    /// `NewPhase`
    pub fn new_phase(&mut self, new_phase: &str) {
        log::info!("phase: {new_phase}");
        self.btm().phase = new_phase.to_string();
        self.update_map_overlay(false);
        self.update_tank_con();
        self.render_encounter(false);
    }

    /// `EncounterMenu`: the tally of the encounter so far; Q saves and
    /// quits mid-encounter.
    pub fn encounter_menu(&mut self) {
        self.darken_screen();
        let exiting = self.campaign.exiting;
        let b = self.battle.as_ref().expect("in a battle");
        let m = &mut self.menu_con;
        m.clear();
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.fg = MENU_TITLE_COLOR;
        m.print_ex(MENU_CON_XM, 1, Align::Center, "Encounter Menu");
        m.fg = color::WHITE;
        if exiting {
            m.print_ex(MENU_CON_XM, 3, Align::Center, &format!("[{}] Return to Main Menu", hl("Enter")));
        } else if b.result != "Undetermined" {
            m.print_ex(MENU_CON_XM, 3, Align::Center, &format!("[{}] Return to Campaign Map", hl("Enter")));
        } else {
            m.print_ex(MENU_CON_XM, 3, Align::Center, &format!("[{}] Return to Game", hl("Enter")));
            m.print_ex(MENU_CON_XM, 4, Align::Center, &format!("[{}] Save Game and Quit", hl("Q")));
        }

        let x = 8;
        m.align = Align::Left;
        m.fg = color::LIGHT_GREY;
        m.print(x, 6, "                                         Destroyed by               Forced off");
        m.print(x, 7, "Enemy Forces              VP Value    Player    Friendly Forces    Encounter Map");
        m.print(x, 8, "-----------------------------------------------------------------------------------");
        m.fg = color::WHITE;
        for (n, line) in [
            "LW and MG Squads             1",
            "Trucks                       1",
            "APC or Armoured Car          2",
            "Self-Propelled Guns          6",
            "Panzer IV                    7",
            "Panzer V (Panther)           9",
            "Panzer VI (Tiger)           12",
            "Anti-Tank Gun                4",
        ]
        .iter()
        .enumerate()
        {
            m.print(x, 9 + n as i32, line);
        }
        const VP_SCORES: [i32; 8] = [1, 1, 2, 6, 7, 9, 12, 4];
        m.align = Align::Right;
        let (mut player_vp, mut friendly_vp, mut left_behind_vp) = (0, 0, 0);
        for n in 0..8 {
            let y = 9 + n as i32;
            m.print(x + 40, y, &b.tank_ko_record[n].to_string());
            player_vp += b.tank_ko_record[n] * VP_SCORES[n];
            m.print(x + 54, y, &b.friendly_ko_record[n].to_string());
            friendly_vp += b.friendly_ko_record[n] * VP_SCORES[n];
            m.print(x + 72, y, &b.left_behind[n].to_string());
            left_behind_vp -= b.left_behind[n] * VP_SCORES[n] / 2;
        }
        m.print(x + 40, 18, &player_vp.to_string());
        m.print(x + 54, 18, &friendly_vp.to_string());
        m.print(x + 72, 18, &left_behind_vp.to_string());
        m.align = Align::Left;
        m.print(x, 18, "Totals:");

        m.fg = color::LIGHT_GREY;
        m.print(x, 20, "Friendly Forces         VP Value      Lost");
        m.print(x, 21, "------------------------------------------");
        m.fg = color::WHITE;
        m.print(x, 22, "Tanks                      -5");
        m.print(x, 23, "Infantry Squads            -3");
        m.align = Align::Right;
        m.print(x + 40, 22, &b.tanks_lost.to_string());
        m.print(x + 40, 23, &b.inf_lost.to_string());

        m.align = Align::Center;
        let vp_total = player_vp + friendly_vp + left_behind_vp - b.tanks_lost * 5 - b.inf_lost * 3;
        m.print(MENU_CON_XM, 25, &format!("Encounter VP Total: {vp_total}"));
        m.print(MENU_CON_XM, 26, &format!("Campaign Day VP: {}    Total Campaign VP: {}", self.campaign.day_vp, self.campaign.vp));
        m.print(MENU_CON_XM, 28, &format!("Encounter Result: {}", b.result));
        m.align = Align::Left;
        self.btm().vp_total = vp_total;
        self.blit_menu();
        self.flush();

        loop {
            self.check_for_event();
            if self.key_vk() == Vk::Enter || self.scr.is_headless() {
                break;
            }
            let kc = self.key_char();
            if self.bt().result == "Undetermined" && (kc == 'q' || kc == 'Q') {
                self.save_game();
                self.campaign.exiting = true;
                self.con.clear();
                return;
            }
            self.flush();
        }
        self.render_encounter(false);
    }

    /// `DoEncounter`: run the encounter to its end.
    pub fn do_encounter(&mut self) {
        loop {
            if self.bt().result != "Undetermined" {
                if !self.campaign.over {
                    self.encounter_menu();
                }
                break;
            }
            if self.campaign.exiting {
                self.battle = None;
                return;
            }
            self.flush();
            self.get_encounter_input();
        }
        let vp = self.bt().vp_total;
        self.campaign.day_vp += vp;
        let text = if vp < 0 {
            format!("You have lost {}", vp.abs())
        } else if vp == 0 {
            "You gain no".to_string()
        } else {
            format!("You are awarded {vp}")
        };
        self.battle = None;
        self.message(&format!("{text} VP for this encounter."));
    }

    /// `GetEncounterInput`: one event, and the phase it moves on.
    pub fn get_encounter_input(&mut self) {
        if self.scr.is_headless() {
            self.autopilot();
            return;
        }
        self.check_for_event();
        let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
        if (mx, my) != self.bt().mouseover {
            self.btm().mouseover = (mx, my);
            self.render_encounter(false);
        }
        if self.scr.mouse.rbutton_pressed {
            self.unit_info(mx, my);
            self.render_encounter(false);
        }

        let vk = self.key_vk();
        let kc = self.key_char();
        match vk {
            Vk::Escape => self.encounter_menu(),
            Vk::F(1) | Vk::Digit(1) => self.show_help(),
            Vk::F(2) | Vk::Digit(2) => {
                self.show_tank_info(false);
            }
            Vk::F(3) | Vk::Digit(3) => self.show_crew_info(),
            Vk::F(4) | Vk::Digit(4) => self.show_settings(),
            Vk::F(5) | Vk::Digit(5) => self.show_campaign_stats(),
            Vk::F(7) | Vk::Digit(7) => {
                self.campaign.sounds = !self.campaign.sounds;
                let text = if self.campaign.sounds { "Sound turned on" } else { "Sound turned off" };
                self.popup(text);
            }
            Vk::Backspace => {
                if self.bt().phase == "Issue Order" {
                    self.btm().selected_order = None;
                    self.new_phase("Orders");
                }
            }
            _ => {}
        }
        if self.campaign.exiting {
            return;
        }

        if vk == Vk::End || vk == Vk::Space || self.bt().trigger_phase {
            self.btm().trigger_phase = false;
            self.next_phase();
            return;
        }

        let phase = self.bt().phase.clone();
        let is = |c: char| kc.eq_ignore_ascii_case(&c);
        match phase.as_str() {
            "Set Spot Sectors" => {
                let Some(sel) = self.bt().selected_crew else { return };
                if is('s') || vk == Vk::Down || is('w') || vk == Vk::Up {
                    let down = is('s') || vk == Vk::Down;
                    let mut c = sel;
                    for _ in 0..self.tank.crew.len() {
                        c = if down { self.tank.next_crew(c) } else { self.tank.prev_crew(c) };
                        if self.crew(c).spot == "Any One Sector" {
                            break;
                        }
                    }
                    self.btm().selected_crew = Some(c);
                    self.update_tank_con();
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                } else if is('d') || vk == Vk::Right {
                    let s = &mut self.crewm(sel).spot_sector;
                    *s = if *s >= 5 { 0 } else { *s + 1 };
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                } else if is('a') || vk == Vk::Left {
                    let s = &mut self.crewm(sel).spot_sector;
                    *s = if *s <= 0 { 5 } else { *s - 1 };
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                }
            }
            "Orders" => {
                let sel = self.bt().selected_crew;
                if is('s') || vk == Vk::Down {
                    if let Some(c) = sel {
                        self.btm().selected_crew = Some(self.tank.next_crew(c));
                    }
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('w') || vk == Vk::Up {
                    if let Some(c) = sel {
                        self.btm().selected_crew = Some(self.tank.prev_crew(c));
                    }
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('h') {
                    self.tank_toggle_hatch(sel);
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('o') {
                    let Some(c) = sel else { return };
                    if self.crew(c).no_actions() {
                        return;
                    }
                    self.new_phase("Issue Order");
                } else if is('r') {
                    self.tank.cycle_reload();
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('t') {
                    self.tank.use_rr = !self.tank.use_rr;
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if kc == 'a' || kc == 'd' || vk == Vk::Left || vk == Vk::Right {
                    let Some(c) = sel else { return };
                    if self.crew(c).no_actions() {
                        return;
                    }
                    let cm = self.crew(c);
                    let n = cm.orders_list.iter().position(|o| o.name == cm.order).unwrap_or(cm.orders_list.len());
                    let len = cm.orders_list.len();
                    if len == 0 {
                        return;
                    }
                    let next = if kc == 'd' || vk == Vk::Right {
                        if n + 1 >= len { 0 } else { n + 1 }
                    } else if n == 0 || n >= len {
                        len - 1
                    } else {
                        n - 1
                    };
                    let name = self.crew(c).orders_list[next].name.clone();
                    self.crewm(c).order = name;
                    self.crew_set_spot_ability(c);
                    self.update_tank_con();
                    self.render_encounter(false);
                }
            }
            "Issue Order" => {
                let Some(c) = self.bt().selected_crew else { return };
                let len = self.crew(c).orders_list.len();
                let sel = self.bt().selected_order.unwrap_or(0);
                if is('w') || vk == Vk::Up {
                    self.btm().selected_order = Some(if sel > 0 { sel - 1 } else { len.saturating_sub(1) });
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('s') || vk == Vk::Down {
                    self.btm().selected_order = Some(if len > sel + 1 { sel + 1 } else { 0 });
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('o') || vk == Vk::Enter {
                    if let Some(order) = self.crew(c).orders_list.get(sel) {
                        let name = order.name.clone();
                        self.crewm(c).order = name.clone();
                        self.crew_set_spot_ability(c);
                        let text = format!("{} now on {name} order.", self.crew(c).name);
                        self.message(&text);
                    }
                    self.btm().selected_order = None;
                    self.new_phase("Orders");
                }
            }
            "Pivot Tank" => {
                if is('d') || vk == Vk::Right {
                    self.tank.new_facing = if self.tank.new_facing == 5 { 0 } else { self.tank.new_facing + 1 };
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                } else if is('a') || vk == Vk::Left {
                    self.tank.new_facing = if self.tank.new_facing == 0 { 5 } else { self.tank.new_facing - 1 };
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                } else if vk == Vk::Enter {
                    self.btm().trigger_phase = true;
                }
            }
            "Rotate Turret" => {
                if is('d') || vk == Vk::Right {
                    self.rotate_turret(true);
                } else if is('a') || vk == Vk::Left {
                    self.rotate_turret(false);
                } else if vk == Vk::Enter {
                    self.btm().trigger_phase = true;
                }
            }
            "Fire Main Gun" => {
                if !self.tank.has_rof {
                    if is('d') || vk == Vk::Right {
                        self.rotate_turret(true);
                    } else if is('a') || vk == Vk::Left {
                        self.rotate_turret(false);
                    } else if is('f') && self.tank.ammo_load == "HE" {
                        let b = self.btm();
                        b.area_fire = !b.area_fire;
                        self.update_tank_con();
                        self.render_encounter(false);
                    }
                    if vk == Vk::Tab {
                        self.select_next_target();
                        self.update_map_overlay(false);
                        self.render_encounter(false);
                    }
                }
                if vk == Vk::Enter {
                    self.fire_main_gun();
                } else if is('r') {
                    self.tank.cycle_reload();
                    self.update_tank_con();
                    self.render_encounter(false);
                } else if is('t') {
                    self.tank.use_rr = !self.tank.use_rr;
                    self.update_tank_con();
                    self.render_encounter(false);
                }
            }
            "Fire MGs" => {
                if self.tank.coax_mg_can_fire {
                    if is('d') || vk == Vk::Right {
                        self.rotate_turret(true);
                    } else if is('a') || vk == Vk::Left {
                        self.rotate_turret(false);
                    }
                }
                let t = &self.tank;
                if (t.coax_mg_can_fire || t.bow_mg_can_fire || t.aa_mg_can_fire) && is('m') {
                    let t = &mut self.tank;
                    t.active_mg = match t.active_mg {
                        0 if t.bow_mg_can_fire => 1,
                        0 if t.aa_mg_can_fire => 2,
                        1 if t.aa_mg_can_fire => 2,
                        1 if t.coax_mg_can_fire => 0,
                        2 if t.coax_mg_can_fire => 0,
                        2 if t.bow_mg_can_fire => 1,
                        n => n,
                    };
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                    self.select_next_target();
                }
                if vk == Vk::Tab {
                    self.select_next_target();
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                } else if vk == Vk::Enter {
                    self.fire_mg();
                    if self.tank.active_mg == -1 {
                        self.btm().trigger_phase = true;
                    }
                }
            }
            _ => {}
        }
    }

    /// Headless play for tests: the gunner fires at whatever he can, the
    /// MGs likewise, and every other phase goes straight on.
    fn autopilot(&mut self) {
        assert!(self.bt().rounds_passed < 500, "encounter does not end");
        let phase = self.bt().phase.clone();
        match phase.as_str() {
            "Orders" => {
                if let Some(g) = self.crew_pos("Gunner") {
                    let pick = ["Fire Main Gun", "Fire Co-Axial MG", "Rotate Turret"][self.rng.int(0, 2) as usize];
                    if self.crew(g).orders_list.iter().any(|o| o.name == pick) {
                        self.crewm(g).order = pick.into();
                    }
                }
                if let Some(d) = self.crew_pos("Driver") {
                    let list: Vec<String> = self.crew(d).orders_list.iter().map(|o| o.name.clone()).collect();
                    if !list.is_empty() {
                        let pick = self.rng.choice(&list).clone();
                        if pick != "Abandon Tank" {
                            self.crewm(d).order = pick;
                        }
                    }
                }
                self.next_phase();
            }
            "Pivot Tank" => {
                self.tank.new_facing = self.rng.int(0, 5);
                self.next_phase();
            }
            "Rotate Turret" => {
                let cw = self.rng.int(0, 1) == 0;
                self.rotate_turret(cw);
                self.next_phase();
            }
            "Fire Main Gun" => {
                if self.bt().target.is_some() && !self.bt().trigger_phase {
                    self.fire_main_gun();
                }
                if self.bt().target.is_none() || self.bt().trigger_phase {
                    self.btm().trigger_phase = false;
                    self.next_phase();
                }
            }
            "Fire MGs" => {
                if self.bt().target.is_some() {
                    self.fire_mg();
                }
                if self.bt().target.is_none() || self.tank.active_mg == -1 {
                    self.btm().trigger_phase = false;
                    self.next_phase();
                }
            }
            _ => self.next_phase(),
        }
    }

    /// The END/Space branch of `GetEncounterInput`: finish the current
    /// phase and run on to the next point where the player has a say.
    fn next_phase(&mut self) {
        let mut phase = self.bt().phase.clone();

        if phase == "Set Spot Sectors" {
            self.new_phase("Spotting");
            self.do_spotting();
            self.new_phase("Orders");
            self.save_game();
            return;
        }

        if phase == "Orders" {
            for i in 0..self.tank.crew.len() {
                if self.crew(i).order == "Abandon Tank" {
                    let text = "You are about to abandon your tank. It will be destroyed. Are you certain?";
                    if self.popup_confirm(text) {
                        self.tank_abandon();
                        return;
                    }
                }
            }
            if self.crew_pos("Loader").is_some_and(|l| self.crew(l).order == "Change Gun Load") {
                self.tank_change_gun_load_menu();
                self.update_tank_con();
                self.render_encounter(false);
                self.save_game();
            }
            if self.setup_pivot() {
                self.new_phase("Pivot Tank");
                self.save_game();
                return;
            }
            phase = "Pivot Tank".into();
            self.btm().phase = phase.clone();
        }

        if phase == "Pivot Tank" {
            if self.crew_pos("Driver").is_some_and(|d| self.crew(d).order == "Pivot Tank") {
                if self.tank.new_facing != 4 {
                    self.pivot_tank();
                } else if self.tank.moving {
                    self.tank.moving = false;
                    self.update_tank_con();
                }
            }
            self.new_phase("Tank Movement");
            self.move_tank();
            if self.setup_rotate_turret() {
                self.new_phase("Rotate Turret");
                self.save_game();
                return;
            }
            phase = "Rotate Turret".into();
            self.btm().phase = phase.clone();
        }

        if phase == "Rotate Turret" {
            self.tank.old_t_facing = self.tank.turret_facing;
            self.update_map_overlay(false);
            if self.setup_main_gun() {
                self.update_map_overlay(false);
                self.render_encounter(false);
                self.save_game();
                return;
            }
            phase = "Fire Main Gun".into();
            self.btm().phase = phase.clone();
        }

        if phase == "Fire Main Gun" {
            self.btm().target = None;
            self.tank.has_rof = false;
            self.update_map_overlay(false);
            self.render_encounter(false);
            for i in 0..self.bt().enemy_units.len() {
                if self.u(i).alive && !self.u(i).hit_record.is_empty() {
                    self.unit_resolve_hits(i);
                }
            }
            if self.setup_fire_mgs() {
                self.save_game();
                return;
            }
            phase = "Fire MGs".into();
            self.btm().phase = phase.clone();
        }

        if phase == "Fire MGs" {
            self.btm().target = None;
            self.new_phase("Smoke Grenades");
            self.handle_smoke_grenades();
            self.new_phase("Smoke Mortar");
            self.handle_smoke_mortar();
            if self.setup_ready_rack() {
                self.popup("Loader may move shells into or out of the Ready Rack");
                self.main_gun_ammo_menu(false);
                self.render_encounter(false);
            }

            self.new_phase("Crew Recovery");
            for i in 0..self.tank.crew.len() {
                self.crew_recovery_roll(i);
            }

            if self.bt().friendly_ambush {
                self.btm().friendly_ambush = false;
            } else {
                self.new_phase("Enemy Action");
                for i in 0..self.bt().enemy_units.len() {
                    if !self.u(i).alive {
                        continue;
                    }
                    self.unit_do_action(i, false);
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                    if self.bt().result != "Undetermined" {
                        return;
                    }
                }
            }

            if !self.bt().all_enemies_dead() {
                self.new_phase("Friendly Action");
                let mut result = false;
                for i in 0..self.bt().enemy_units.len() {
                    if !self.u(i).alive {
                        continue;
                    }
                    if self.unit_friendly_action(i, false, false, false, false) {
                        result = true;
                    }
                    self.update_map_overlay(false);
                    self.render_encounter(false);
                }
                if !result {
                    self.popup("No results from Friendly Action.");
                }
            }

            self.new_phase("Random Events");
            self.random_event();
            self.new_phase("Attempt Repairs");
            self.attempt_repairs(false);

            if !self.tank.alive {
                return;
            }
            if self.bt().all_enemies_dead() {
                self.message("You have won the encounter!");
                self.render_encounter(false);
                self.btm().result = "Victory".into();
                return;
            }

            self.btm().rounds_passed += 1;
            self.tank.reset();
            for u in &mut self.btm().enemy_units {
                if u.alive {
                    u.shot_at = false;
                }
            }
            if self.bt().battle_leadership {
                self.message("Battle Leadership no longer in effect.");
                self.btm().battle_leadership = false;
            }
            if self.pos_skill_check("Commander", "Battle Leadership") {
                self.message("Battle Leadership now in effect.");
                self.btm().battle_leadership = true;
            }
            self.new_phase("Smoke Depletion");
            self.deplete_smoke();
            self.calculate_smoke_factors();
            for i in 0..self.tank.crew.len() {
                self.crew_build_orders_list(i, false);
                self.crew_set_spot_ability(i);
            }
            self.spot_sectors_or_spotting();
            self.save_game();
        }
    }

    /// Choose spot sectors if anyone needs to, or go straight to spotting
    /// and orders.
    fn spot_sectors_or_spotting(&mut self) {
        if self.check_spot_sectors() {
            let first = self.tank.crew.iter().position(|c| c.spot == "Any One Sector");
            self.btm().selected_crew = first;
            self.new_phase("Set Spot Sectors");
        } else {
            self.do_spotting();
            self.new_phase("Orders");
        }
    }

    /// `InitEncounter`: start an encounter (or carry on a loaded one) and
    /// run it.
    pub fn init_encounter(&mut self, load: bool, counterattack: bool) {
        if !load {
            self.battle = Some(Battle::new(counterattack));
        }
        self.rescale_battle();
        if load {
            self.update_date_con();
            self.update_tank_con();
            self.update_msg_con();
            self.paint_map_con();
            self.update_map_overlay(false);
            self.render_encounter(false);
        } else {
            self.tank_set_deployment();
            for i in 0..self.tank.crew.len() {
                self.crew_build_orders_list(i, false);
                self.crew_set_spot_ability(i);
            }
            self.paint_map_con();
            self.update_map_overlay(false);
            self.update_tank_con();
            self.update_msg_con();
            self.update_date_con();
            self.render_encounter(false);
            self.zoom_in();

            self.message("Encounter begins!");
            self.activate_enemies();
            self.render_encounter(false);

            let node = self.campaign.player_node().clone();
            if node.arty_strike {
                self.campaign.player_node_mut().arty_strike = false;
                self.popup("Friendly forces conduct artillery fire against the enemy.");
                self.play_sound("arty_firing");
                self.all_friendly_action(false, true, false, false, "No results from Friendly Artillery.");
            } else if node.air_strike {
                self.campaign.player_node_mut().air_strike = false;
                self.popup("Friendly forces conduct an air strike against the enemy.");
                self.all_friendly_action(false, false, true, false, "No results from air strike.");
            }
            if node.advancing_fire {
                self.campaign.player_node_mut().advancing_fire = false;
                self.popup("You use advancing fire to attack the enemy.");
                if let Some(s) = firing_sound(self.tank.stats.str("main_gun")) {
                    self.play_sound(s);
                    self.wait(300);
                    self.play_sound(s);
                    self.wait(300);
                    self.play_sound(s);
                }
                self.all_friendly_action(false, false, false, true, "No results from advancing fire.");
            }

            self.btm().selected_crew = Some(0);

            if !self.bt().all_enemies_dead() {
                let mut roll = self.roll_1d10();
                if self.campaign.weather.fog || self.campaign.weather.precip != "None" {
                    roll -= 1;
                }
                if self.campaign.player_node().node_type == "F" {
                    roll -= 2;
                }
                if self.pos_skill_check("Commander", "Keen Senses") {
                    if self.campaign.scen_type != "Counterattack" && !self.bt().counterattack {
                        roll += 2;
                    } else {
                        roll -= 2;
                    }
                }
                if roll <= 7 {
                    if self.campaign.scen_type == "Counterattack" || self.bt().counterattack {
                        self.popup("Your forces have ambushed the enemy!");
                        self.btm().friendly_ambush = true;
                    } else {
                        self.popup("Your tank has been ambushed! Enemy gets first attack.");
                        self.new_phase("Enemy Action");
                        for i in 0..self.bt().enemy_units.len() {
                            if !self.u(i).alive {
                                continue;
                            }
                            self.unit_do_action(i, true);
                            self.update_map_overlay(false);
                            self.render_encounter(false);
                            if self.bt().result != "Undetermined" || self.campaign.over {
                                if !self.campaign.over {
                                    self.encounter_menu();
                                }
                                // the original returns with the battle still
                                // set (its pop-ups then redraw the encounter
                                // over the campaign) and never counts this
                                // encounter's VP; the VP is kept as it was
                                self.battle = None;
                                return;
                            }
                        }
                    }
                    self.new_phase("Random Events");
                    self.random_event();
                } else {
                    self.popup("Enemy units are caught off guard, you have first attack.");
                }
                for i in 0..self.tank.crew.len() {
                    self.crew_set_spot_ability(i);
                }
                self.spot_sectors_or_spotting();
            } else {
                self.popup("Your initial attack has destroyed all enemy forces!");
                self.new_phase("Orders");
            }
            self.save_game();
        }
        self.do_encounter();
    }

    /// The opening artillery, air strike or advancing fire, against every
    /// enemy unit.
    fn all_friendly_action(&mut self, flanking: bool, artillery: bool, air: bool, advance: bool, none_text: &str) {
        let mut result = false;
        for i in 0..self.bt().enemy_units.len() {
            if !self.u(i).alive {
                continue;
            }
            if self.unit_friendly_action(i, flanking, artillery, air, advance) {
                result = true;
            }
            self.update_map_overlay(false);
            self.render_encounter(false);
        }
        if !result {
            self.popup(none_text);
        }
    }

    /// `RenderEncounter(zoom_in=True)`: a frame opening out from the
    /// player's area on the campaign map onto the encounter.
    fn zoom_in(&mut self) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let (x, y) = match &self.campaign.day_map {
            Some(m) => {
                let (x, y) = self.node_screen_pos(m.player_node);
                (x, y.clamp(1, sh() - 1))
            }
            None => (sxm(), sym()),
        };
        let screen = self.root.clone();
        let mut w = 3;
        while w < sw() {
            crate::console::Console::blit(&screen, x - w, y - w / 2, w * 2, w, &mut self.root, x - w, y - w / 2);
            self.root.fg = color::WHITE;
            self.root.frame(x - w, y - w / 2, w * 2, w, false, false);
            self.flush();
            if x - w < 0 && x + w >= sw() {
                break;
            }
            w += 2;
        }
        self.render_encounter(false);
    }

    /// `SaveGame`: the campaign, tank and battle as they stand; not once
    /// the campaign is over.
    pub fn save_game(&mut self) {
        if self.campaign.over {
            return;
        }
        crate::save::write(&self.save_info(), &self.campaign, &self.tank, &self.battle);
        if self.test_saves.is_some() {
            if let Ok(text) = crate::save::to_string(&self.save_info(), &self.campaign, &self.tank, &self.battle) {
                self.test_saves.as_mut().expect("test saves").push(text);
            }
        }
    }

    /// `SavedGameInfo` for the game as it stands.
    pub fn save_info(&self) -> crate::save::Info {
        let commander = self.crew_pos("Commander").map(|c| self.crew(c).name.clone()).unwrap_or_default();
        crate::save::Info {
            format: crate::save::FORMAT,
            game_version: VERSION.into(),
            campaign_name: self.campaign.campaign_name.clone(),
            commander_name: commander,
            tank_name: self.tank.name.clone(),
            current_date: self.campaign.get_date(None),
        }
    }

    /// `LoadGame`: the saved campaign, tank and battle, with the campaign
    /// file read again (`LoadCampaignInfo`).
    pub fn load_game(&mut self) -> Result<(), String> {
        let (campaign, tank, battle) = crate::save::load()?;
        self.restore(campaign, tank, battle)
    }

    pub fn restore(&mut self, campaign: Campaign, tank: PlayerTank, battle: Option<Battle>) -> Result<(), String> {
        let file = campaign.campaign_file.clone();
        let def = defs::CAMPAIGN_FILES
            .iter()
            .find(|(f, _)| *f == file)
            .ok_or_else(|| format!("unknown campaign file {file}"))
            .and_then(|(_, xml)| defs::load_campaign(xml))?;
        self.campaign = campaign;
        self.campaign.load_info(&def);
        self.campaign.exiting = false;
        self.tank = tank;
        self.battle = battle;
        self.rescale_battle();
        log::info!("loaded: {} on {}", self.campaign.campaign_name, self.campaign.get_date(None));
        Ok(())
    }

    /// The encounter's own debug start (`armcom --encounter`): a crew, a
    /// tank and one area of a Patton's Best day, straight into battle.
    pub fn debug_encounter(&mut self, scen_type: &str) {
        let def = defs::load_campaign(defs::CAMPAIGN_FILES[1].1).expect("campaign file");
        self.campaign.campaign_file = "pattons_best.xml".into();
        self.campaign.load_info(&def);
        self.campaign.current_date = [1944, 8, 2];
        self.campaign.scen_type = scen_type.to_string();
        self.campaign.scen_res = "Medium".into();
        self.campaign.hour = 10;
        self.campaign.minute = 15;
        let mut node = MapNode::new(10, 10);
        node.node_type = "A".into();
        node.resistance = Some("Medium".into());
        self.campaign.day_map = Some(CampaignDayMap { nodes: vec![node], ..Default::default() });
        self.campaign.day_in_progress = true;
        self.weather_generate_new();

        self.tank = PlayerTank::new(Some("M4A1 Turret B"));
        self.set_tank_stats();
        self.tank.name = self.rng.choice(&defs::defs().tank_names).clone();
        for (pos, rank) in [("Commander", 3), ("Gunner", 2), ("Loader", 1), ("Driver", 1), ("Asst. Driver", 0)] {
            self.spawn_crew_member(None, pos, rank);
        }
        let t = &mut self.tank;
        t.general_ammo.set("HE", 30);
        t.general_ammo.set("AP", 45);
        t.general_ammo.set("WP", 6);
        t.rr_ammo.set("HE", 3);
        t.rr_ammo.set("AP", 4);
        t.rr_ammo.set("WP", 1);
        t.ammo_load = "AP".into();
        t.ammo_reload = "AP".into();
        self.init_encounter(false, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::ui::Screen;

    /// Whole encounters, headless, many seeds and all three missions:
    /// they must end, and nothing may panic on the way.
    #[test]
    fn encounters_play_to_the_end() {
        let mut results = std::collections::BTreeMap::new();
        for seed in 1..=150u64 {
            for scen in ["Advance", "Battle", "Counterattack"] {
                let mut g = Game::new(Screen::headless(), Rng::new(seed * 7919));
                g.campaign.animations = false;
                g.campaign.pause_labels = false;
                g.debug_encounter(scen);
                assert!(g.battle.is_none(), "seed {seed} {scen}: encounter left open");
                let outcome = if !g.tank.alive { "tank lost" } else if g.campaign.exiting { "exiting" } else { "won" };
                *results.entry(outcome).or_insert(0) += 1;
            }
        }
        eprintln!("{results:?}");
        assert!(results.get("won").copied().unwrap_or(0) > 0);
    }
}
