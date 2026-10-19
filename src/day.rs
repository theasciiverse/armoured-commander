//! "Campaign Functions" in armcom.py that make up a day of action:
//! checking and entering areas, strikes, waiting for a counterattack,
//! resupply, what follows a battle (`PostEncounter`, `ReplaceCrew`), the
//! tank and ammunition screens, the campaign menu, and the day itself
//! (`InitCampaignDay`, `DoCampaignDay`).

use crate::color;
use crate::console::{Align, Console};
use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::model::*;
use crate::ui::Vk;

/// `CREW_ORDER` in `ReplaceCrew`: the crew list's order.
const CREW_ORDER: [&str; 5] = ["Commander", "Gunner", "Loader", "Driver", "Asst. Driver"];

impl Game {
    /// `SetupCheckArea`
    pub fn setup_check_area(&mut self) {
        if self.campaign.selected_node.is_none() {
            self.campaign.selected_node = self.campaign.player_node().links.first().copied();
        }
        self.campaign.input_mode = "Check Adjacent Area".into();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `CheckArea`: learn the selected area's resistance.
    pub fn check_area(&mut self) {
        let Some(s) = self.campaign.selected_node else { return };
        let n = &self.campaign.day_map().nodes[s];
        if n.friendly_control || n.res_known {
            return;
        }
        let Some(res) = n.resistance.clone() else { return };
        self.campaign.day_map_mut().nodes[s].res_known = true;

        let (x, y) = self.node_screen_pos(s);
        self.show_label(x, y, &format!("{res} enemy resistance reported in this area."), None);
        if !self.campaign.free_check {
            self.spend_time(0, 15);
        } else {
            self.campaign.free_check = false;
        }
        self.campaign.input_mode = "None".into();

        if res == "Heavy" && self.roll_1d10() <= 2 {
            let text = self.rng.choice(&defs::defs().crew_talk_heavy_res).clone();
            self.crew_talk(&text, None);
        }

        if self.campaign.day_map().nodes[s].quest_type.as_deref() == Some("RECON") {
            self.campaign.add_stat("Quests Completed", 1);
            let bonus = self.campaign.day_map().nodes[s].quest_vp_bonus.unwrap_or(0);
            self.popup(&format!(
                "Congratulations, commander. You have reported your reconnaissance of the requested map area and \
                 have earned {bonus} bonus VP."
            ));
            self.write_journal("RECON quest completed");
            self.campaign.day_vp += bonus;
            let n = &mut self.campaign.day_map_mut().nodes[s];
            n.quest_type = None;
            n.quest_vp_bonus = None;
            self.campaign.quest_active = false;
        }

        self.save_game();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `SelectNextArea`: the next adjacent area clockwise (Shift for
    /// anticlockwise). In a counterattack only friendly areas on the front
    /// line can be chosen; the original listed such an area once for each
    /// enemy area beside it, so Tab could stick on it, and lists it once
    /// here.
    pub fn select_next_area(&mut self) {
        let map = self.campaign.day_map();
        let player = &map.nodes[map.player_node];
        let nodes: Vec<usize> = if self.campaign.scen_type == "Counterattack" {
            player
                .links
                .iter()
                .copied()
                .filter(|&l| {
                    let n = &map.nodes[l];
                    n.friendly_control && (n.top_edge || n.links.iter().any(|&l2| !map.nodes[l2].friendly_control))
                })
                .collect()
        } else {
            player.links.clone()
        };
        if nodes.is_empty() {
            self.campaign.selected_node = None;
            return;
        }
        let Some(selected) = self.campaign.selected_node else {
            self.campaign.selected_node = Some(nodes[0]);
            return;
        };

        // by compass heading from the player's area, north first
        let heading = |i: usize| {
            let n = &map.nodes[i];
            let rads = ((n.y - player.y) as f64).atan2((n.x - player.x) as f64).rem_euclid(2.0 * std::f64::consts::PI);
            let mut degs = rads.to_degrees() + 90.0;
            if degs >= 360.0 {
                degs -= 360.0;
            }
            degs as i32
        };
        let mut list: Vec<(usize, i32)> = nodes.iter().map(|&i| (i, heading(i))).collect();
        list.sort_by_key(|&(_, h)| h);
        let n = list.iter().position(|&(i, _)| i == selected).unwrap_or(list.len());
        let new = if self.scr.key.shift {
            if n == 0 { list[list.len() - 1] } else { list[n - 1] }
        } else if n + 1 < list.len() {
            list[n + 1]
        } else {
            list[0]
        };
        self.campaign.selected_node = Some(new.0);
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `SetupMoveArea`
    pub fn setup_move_area(&mut self) {
        if self.campaign.selected_node.is_none() {
            self.select_next_area();
        }
        if self.campaign.selected_node.is_none() {
            return;
        }
        self.campaign.input_mode = "Move Into Adjacent Area".into();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `MoveArea`: move into the selected area, perhaps with advancing
    /// fire, and meet what is there.
    pub fn move_area(&mut self) {
        let Some(sel) = self.campaign.selected_node else { return };
        let player = self.campaign.day_map().player_node;
        let p = &self.campaign.day_map().nodes[player];
        let mut text = "You move into a new area".to_string();
        let mut time_req = if p.stone_road_links.contains(&sel) {
            text.push_str(" along an improved road");
            STONE_ROAD_MOVE_TIME
        } else if p.dirt_road_links.contains(&sel) {
            text.push_str(" along a dirt road");
            DIRT_ROAD_MOVE_TIME
        } else {
            NO_ROAD_MOVE_TIME
        };
        let w = &self.campaign.weather;
        if w.ground != "Dry" || w.precip != "None" || w.fog {
            time_req += GROUND_MOVE_TIME_MODIFIER;
        }
        text.push_str(&format!(" which takes {time_req} minutes."));

        // advancing fire needs a working main gun and at least 6 HE
        let t = &self.tank;
        let gun_malfunction = t.has_damage("Main Gun Malfunction") || t.has_damage("Main Gun Broken");
        let total_he = t.general_ammo.get("HE") + t.rr_ammo.get("HE");
        let friendly_control = self.campaign.day_map().nodes[sel].friendly_control;
        if !gun_malfunction && !friendly_control && total_he >= 6 {
            text.push_str(&format!(" Use advancing fire (requires 1-6 HE rounds; currently have {total_he} rounds)?"));
            if self.popup_confirm(&text) {
                let rounds_req = self.roll_1d6();
                for _ in 0..rounds_req {
                    let t = &mut self.tank;
                    if t.general_ammo.get("HE") > 0 {
                        t.general_ammo.add("HE", -1);
                    } else if t.rr_ammo.get("HE") > 0 {
                        t.rr_ammo.add("HE", -1);
                    }
                }
                self.campaign.day_map_mut().nodes[sel].advancing_fire = true;
                let s = if rounds_req > 1 { "s" } else { "" };
                self.popup(&format!("You expend {rounds_req} HE round{s} entering the area."));
            }
        } else {
            self.popup(&text);
        }

        self.spend_time(0, time_req);
        self.play_sound("sherman_movement");

        if self.campaign.animations && !self.scr.is_headless() {
            let (a, b) = (&self.campaign.day_map().nodes[player], &self.campaign.day_map().nodes[sel]);
            for (x, y) in get_line(a.x, a.y, b.x, b.y) {
                self.update_c_overlay_anim(None, Some((x, y)));
                self.render_campaign(false);
                self.wait(100);
            }
        }

        self.campaign.day_map_mut().player_node = sel;
        self.campaign.selected_node = None;
        self.campaign.input_mode = "None".into();
        self.update_c_overlay(None);
        self.update_c_action_con();
        self.render_campaign(false);

        let node = self.campaign.player_node().clone();
        if self.campaign.scen_type == "Counterattack" || !node.friendly_control {
            let mut roll = self.roll_1d10();
            match node.node_type.as_str() {
                "A" => roll += 1,
                "C" | "F" => roll += 2,
                _ => {}
            }
            let mut target_score = match node.resistance.as_deref() {
                Some("Light") => 8,
                Some("Medium") => 6,
                _ => 4,
            };
            if let Some(q) = &node.quest_type {
                if q == "CAPTURE" || q == "RESCUE" {
                    target_score = 1;
                }
            } else if self.campaign.scen_type == "Counterattack" {
                target_score = 1;
            }

            let no_combat = if roll < target_score {
                self.popup("You meet no resistance in this area.");
                for i in 0..self.tank.crew.len() {
                    self.crewm(i).award_exp(1);
                }
                if self.roll_1d10() == 1 {
                    let text = self.rng.choice(&defs::defs().crew_talk_no_res).clone();
                    self.crew_talk(&text, None);
                }
                true
            } else {
                self.battle_triggered();
                self.init_encounter(false, false);
                if self.campaign.exiting {
                    return;
                }
                false
            };
            self.post_encounter(no_combat);
        } else {
            if node.quest_type.as_deref() == Some("DEFEND") {
                let m = *self.rng.choice(&[15, 30, 45]);
                self.popup(&format!("You arrive to defend the map area. {m} minutes later, the expected attack occurs."));
                self.spend_time(0, m);
                self.init_encounter(false, true);
                if self.campaign.exiting {
                    return;
                }
                self.post_encounter(false);
                return;
            }
            self.check_sunset();
            self.save_game();
            self.update_c_action_con();
            self.update_c_overlay(None);
            self.render_campaign(false);
        }
    }

    /// The pop-up and journal entries before a battle, and its 15 minutes.
    fn battle_triggered(&mut self) {
        self.popup("A battle encounter is triggered!");
        self.write_journal("");
        let text = format!(
            "Battle encounter triggered at {}:{:02}, {} terrain",
            self.campaign.hour,
            self.campaign.minute,
            self.campaign.terrain_desc(self.campaign.player_node())
        );
        self.write_journal(&text);
        self.spend_time(0, 15);
    }

    /// `AwaitEnemy`: wait for the next attack in a counterattack; with
    /// `no_time`, it comes at once.
    pub fn await_enemy(&mut self, no_time: bool) {
        if !no_time {
            self.popup("You await an enemy counterattack.");
            let (_, _, mut roll) = self.roll_2d6();
            match self.campaign.scen_res.as_str() {
                "Medium" => roll += 2,
                "Heavy" => roll += 4,
                _ => {}
            }
            let (h, m) = if roll >= 11 {
                (1, 15)
            } else if roll >= 8 {
                (1, 30)
            } else if roll >= 6 {
                (1, 45)
            } else if roll >= 4 {
                (2, 0)
            } else {
                (2, 30)
            };
            self.spend_time(h, m);
            self.check_sunset();
            if self.campaign.sunset {
                return;
            }
        }

        self.battle_triggered();
        // the original picks the attack's resistance from an enemy area
        // beside the player, but `InitEncounter` never uses it
        let map = self.campaign.day_map();
        let enemy: Vec<usize> = map.nodes[map.player_node].links.iter().copied().filter(|&l| !map.nodes[l].friendly_control).collect();
        if !enemy.is_empty() {
            self.rng.choice(&enemy);
        }
        self.init_encounter(false, false);
        if self.campaign.exiting {
            return;
        }
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
        self.post_encounter(false);
        self.check_sunset();
        self.save_game();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `SetupCallStrike`
    pub fn setup_call_strike(&mut self) {
        if self.campaign.selected_node.is_none() {
            self.select_next_area();
        }
        if self.campaign.selected_node.is_none() {
            return;
        }
        self.campaign.input_mode = "Call in Strike".into();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `CallStrike`: artillery (`a`) or an air strike (`r`) on the selected
    /// area; each success makes the next less likely.
    pub fn call_strike(&mut self, key_char: char) {
        let Some(s) = self.campaign.selected_node else { return };
        let n = &self.campaign.day_map().nodes[s];
        let w = &self.campaign.weather;
        let air = key_char.eq_ignore_ascii_case(&'r');
        if n.friendly_control || (air && (w.clouds == "Overcast" || w.fog || w.precip == "Snow")) {
            return;
        }
        if n.air_strike || n.arty_strike {
            return;
        }
        self.campaign.input_mode = "None".into();

        let (_, _, roll) = self.roll_2d6();
        let mut success = false;
        let text = if !air {
            self.spend_time(0, 15);
            if roll <= self.campaign.arty_chance {
                success = true;
                self.campaign.day_map_mut().nodes[s].arty_strike = true;
                if self.campaign.arty_chance > 2 {
                    self.campaign.arty_chance -= 1;
                }
                "Success: Friendly artillery strikes target area"
            } else {
                "Friendly artillery is unable to strike target area"
            }
        } else {
            self.spend_time(0, 30);
            if roll <= self.campaign.air_chance {
                success = true;
                self.campaign.day_map_mut().nodes[s].air_strike = true;
                if self.campaign.air_chance > 2 {
                    self.campaign.air_chance -= 1;
                }
                "Success: Friendly air forces strike target area"
            } else {
                "Friendly air forces are unable to strike target area"
            }
        };
        let (x, y) = self.node_screen_pos(s);
        self.show_label(x, y, text, None);

        if success {
            self.wait(400);
            self.blit_con();
            self.flush();
            self.play_sound("arty_firing");
            self.campaign_strike_animation(x, y);
            if !air && self.roll_1d10() <= 3 {
                let text = self.rng.choice(&defs::defs().crew_talk_arty_strike).clone();
                self.crew_talk(&text, None);
            }
        } else if !air && self.roll_1d10() <= 3 {
            let text = self.rng.choice(&defs::defs().crew_talk_no_arty_strike).clone();
            self.crew_talk(&text, None);
        }

        self.update_c_action_con();
        let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
        self.update_c_info_con(mx, my);
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// `ArtyStrikeAnimation` on the day map: bursts scattered round the
    /// area's centre, drawn straight onto the screen.
    fn campaign_strike_animation(&mut self, x: i32, y: i32) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let screen = self.root.clone();
        for _ in 0..10 {
            let x1 = x + self.rng.int(-3, 3);
            let y1 = y + self.rng.int(-2, 2);
            for (ch, col, ms) in [(42u8, color::YELLOW, 60u64), (15, color::LIGHT_YELLOW, 60), (176, color::LIGHT_GREY, 80)] {
                if x1 >= C_MAP_CON_X && y1 >= C_MAP_CON_Y {
                    self.root.put_char_ex(x1, y1, ch, col, color::BLACK);
                }
                self.flush();
                self.wait(ms);
            }
            self.root = screen.clone();
        }
        self.flush();
    }

    /// `PostEncounter`: after a battle or an area taken without one:
    /// repairs, capture and quests, crew replacements, sunset, and the next
    /// map or event.
    pub fn post_encounter(&mut self, no_combat: bool) {
        if self.check_commander() {
            self.campaign.exiting = true;
        }
        if !self.campaign.exiting && !no_combat {
            self.attempt_repairs(true);
        }

        if self.campaign.exiting || !self.tank.alive || self.tank.swiss_cheese {
            self.campaign.sunset = true;
            if !self.tank.alive {
                self.write_journal("Player tank knocked out, action day is over");
            } else if self.tank.swiss_cheese {
                self.write_journal("Player tank damaged beyond repair, action day is over");
            } else {
                return;
            }
            self.end_of_day();
            return;
        }

        self.tank_reset_after_encounter();

        let player = self.campaign.day_map().player_node;
        if !self.campaign.player_node().friendly_control {
            self.campaign.player_node_mut().friendly_control = true;
            self.award_capture_vp(player, false);
            self.campaign.add_stat("Map Areas Captured", 1);
            if let Some(q) = self.campaign.player_node().quest_type.clone() {
                if q == "CAPTURE" || q == "RESCUE" {
                    self.campaign.add_stat("Quests Completed", 1);
                    let bonus = self.campaign.player_node().quest_vp_bonus.unwrap_or(0);
                    let rescued = if q == "RESCUE" { " and rescued your allied units" } else { "" };
                    self.popup(&format!(
                        "Congratulations, commander. You have captured the requested map area{rescued}. You have \
                         earned {bonus} bonus VP."
                    ));
                    self.write_journal(&format!("{q} quest completed."));
                    self.campaign.day_vp += bonus;
                }
                // this cancels a RECON quest on the area too
                let n = self.campaign.player_node_mut();
                n.quest_type = None;
                n.quest_vp_bonus = None;
                n.quest_time_limit = None;
                self.campaign.quest_active = false;
            }
        } else if self.campaign.player_node().quest_type.as_deref() == Some("DEFEND") {
            self.campaign.add_stat("Quests Completed", 1);
            self.popup("Congratulations, commander. You have defended the requested map area");
            self.write_journal("DEFEND quest completed.");
            self.campaign.day_vp += self.campaign.player_node().quest_vp_bonus.unwrap_or(0);
            let n = self.campaign.player_node_mut();
            n.quest_type = None;
            n.quest_vp_bonus = None;
            self.campaign.quest_active = false;
        } else if !no_combat && self.campaign.scen_type == "Counterattack" {
            self.award_capture_vp(player, true);
        }

        for i in 0..self.tank.crew.len() {
            self.crewm(i).award_exp(1);
        }
        self.replace_crew();

        if self.tank.immobilized {
            self.write_journal("Player tank immobilized, action day is over");
            self.campaign.sunset = true;
            for i in 0..self.tank.crew.len() {
                let (_, _, roll) = self.roll_2d6();
                self.crewm(i).award_exp(roll);
            }
            self.campaign_menu();
            return;
        }

        self.check_sunset();

        if !self.campaign.sunset && !no_combat {
            let text = format!("Battle encounter ended at {}:{:02}", self.campaign.hour, self.campaign.minute);
            self.write_journal(&text);
            self.popup("Set up your tank for the next battle.");
            self.campaign_view_tank(false);
            self.render_campaign(false);
        }

        self.build_action_list();
        self.update_c_action_con();

        if !self.campaign.sunset {
            if self.campaign.scen_type != "Counterattack" && self.campaign.player_node().exit {
                self.popup("You captured the exit area! Press Enter to move to new map.");
                self.write_journal("Captured exit area and moved to new map");
                self.generating_map_screen();
                while !self.generate_campaign_map() {}
                let p = self.campaign.day_map().player_node;
                self.move_view_to(Some(p));
                self.setup_check_area();
                self.campaign.free_check = true;
                self.render_campaign(false);
                self.popup("Select an adjacent area to check for enemy resistance.");
            } else if self.campaign.scen_type == "Counterattack" {
                self.do_enemy_advance();
            } else {
                self.random_campaign_event();
            }
        }

        self.save_game();
        self.update_date_con();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// "Generating Campaign Map..." on a blank screen.
    fn generating_map_screen(&mut self) {
        self.con.bg = color::BLACK;
        self.con.clear();
        self.con.print_ex(SCREEN_XM, SCREEN_HEIGHT / 2, Align::Center, "Generating Campaign Map...");
        self.blit_con();
        self.flush();
    }

    /// `GetHighestCrewLevel`: the cap on a replacement's level.
    pub fn get_highest_crew_level(&self) -> i32 {
        self.tank.crew.iter().filter(|c| c.alive && !c.v_serious_wound).map(|c| c.level).max().unwrap_or(1)
    }

    /// `ReplaceCrew`: the dead and the very seriously wounded are replaced;
    /// replacing anyone takes 30 minutes on a day of action, except in a
    /// counterattack.
    pub fn replace_crew(&mut self) {
        let mut replaced = false;
        let highest_level = self.get_highest_crew_level();
        let mut i = 0;
        while i < self.tank.crew.len() {
            let c = self.crew(i);
            if !c.alive || c.v_serious_wound {
                let what = if !c.alive { "killed in action." } else { "severely wounded and has been sent home." };
                let (position, rank_level, old_name) = (c.position.clone(), c.rank_level, c.name.clone());
                let text =
                    format!("{old_name} has been {what} A final report on their service will be added to the campaign journal.");
                self.popup(&text);
                for line in self.crew_generate_report(i) {
                    self.write_journal(&line);
                }
                replaced = true;
                if let Some(n) = self.spawn_crew_member(None, &position, rank_level) {
                    let level = self.rng.int(1, highest_level);
                    self.crew_set_level(n, level);
                    let text = format!("{old_name} is replaced by {} in the {}'s position", self.crew(n).name, self.crew(n).position);
                    self.popup(&text);
                    self.write_journal(&text);
                    self.show_skills(n);
                }
            } else {
                let c = self.crewm(i);
                c.stunned = false;
                c.unconscious = false;
            }
            i += 1;
        }

        for i in (0..self.tank.crew.len()).rev() {
            let c = self.crew(i);
            if !c.alive || c.v_serious_wound {
                let stat = if !c.alive { "Crewmen KIA" } else { "Crewmen Sent Home" };
                self.campaign.add_stat(stat, 1);
                self.tank.crew.remove(i);
            }
        }
        self.tank.crew.sort_by_key(|c| CREW_ORDER.iter().position(|p| *p == c.position));
        if self.campaign.selected_crew.is_some_and(|s| s >= self.tank.crew.len()) {
            self.campaign.selected_crew = Some(0);
        }

        if replaced && self.campaign.day_in_progress && self.campaign.scen_type != "Counterattack" {
            self.popup("Crew replacement took 30 mins");
            self.spend_time(0, 30);
        }
    }

    /// `SetupResupply`: an hour's wait (15 minutes in a counterattack, where
    /// the trucks always come) for the supply trucks.
    pub fn setup_resupply(&mut self) {
        let mut roll = self.roll_1d10();
        let time_used = if self.campaign.scen_type == "Counterattack" {
            roll = 1;
            15
        } else {
            60
        };
        self.spend_time(0, time_used);
        if roll <= 7 {
            self.popup("Supply trucks arrive; you may replenish your ammo stores.");
            self.tank.smoke_grenades = 6;
            self.tank.smoke_bombs = 15;
            self.campaign.resupply = true;
            self.main_gun_ammo_menu(false);
            self.campaign.resupply = false;
        } else {
            self.popup("You wait for resupply but the supply trucks are delayed and never arrive.");
        }
        self.update_c_action_con();
        self.render_campaign(false);
        self.check_sunset();
    }

    /// `CampaignViewTank`: the tank and crew, to set hatches, gun load and
    /// ammunition; `load_ammo_menu` opens the ammunition menu straight away.
    pub fn campaign_view_tank(&mut self, mut load_ammo_menu: bool) {
        self.darken_screen();
        let con_x = MENU_CON_XM - TANK_CON_WIDTH / 2;
        let con_y = 3;
        if self.campaign.resupply {
            self.tank.smoke_grenades = 6;
            self.tank.smoke_bombs = 15;
        }
        if self.campaign.selected_crew.is_none() {
            self.campaign.selected_crew = Some(0);
        }
        if self.scr.is_headless() {
            if load_ammo_menu {
                self.main_gun_ammo_menu(true);
            }
            return;
        }

        loop {
            self.update_tank_con();
            let m = &mut self.menu_con;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Player Tank View");
            m.fg = color::WHITE;
            Console::blit(&self.tank_con, 0, 0, TANK_CON_WIDTH, TANK_CON_HEIGHT, m, con_x, con_y);
            let y = con_y + TANK_CON_HEIGHT + 2;
            m.print_ex(
                MENU_CON_XM,
                y,
                Align::Center,
                &format!("Change [{}]un load, Ammo [{}]eload, [{}]oggle Ready Rack use", hl("G"), hl("R"), hl("T")),
            );
            m.print_ex(
                MENU_CON_XM,
                y + 1,
                Align::Center,
                &format!("[{}] or [{}]: move crew selection, [{}] toggle their hatch", hl("W/S"), hl("Up/Down"), hl("H")),
            );
            m.print_ex(MENU_CON_XM, y + 2, Align::Center, &format!("Open [{}]ain Gun Ammunition Menu", hl("M")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] to confirm this loadout", hl("Enter")));
            self.blit_menu();
            self.flush();

            let mut refresh = false;
            while !refresh {
                self.flush();
                self.check_for_event();
                if self.key_vk() == Vk::Enter {
                    if self.campaign.resupply && self.campaign.day_in_progress {
                        let total: i32 = AMMO_TYPES.iter().map(|a| self.tank.general_ammo.get(a)).sum();
                        if total == 0 {
                            let text = "You have not loaded any ammo, are you sure you want to continue?";
                            if !self.popup_ex(text, true, true) {
                                refresh = true;
                                continue;
                            }
                        }
                    }
                    return;
                }
                let kc = self.key_char().to_ascii_lowercase();
                let vk = self.key_vk();
                let sel = self.campaign.selected_crew.unwrap_or(0);
                if kc == 's' || vk == Vk::Down {
                    self.campaign.selected_crew = Some(self.tank.next_crew(sel));
                    refresh = true;
                } else if kc == 'w' || vk == Vk::Up {
                    self.campaign.selected_crew = Some(self.tank.prev_crew(sel));
                    refresh = true;
                } else if kc == 'h' {
                    self.tank_toggle_hatch(Some(sel));
                    refresh = true;
                } else if kc == 'r' {
                    self.tank.cycle_reload();
                    refresh = true;
                } else if kc == 't' {
                    self.tank.use_rr = !self.tank.use_rr;
                    refresh = true;
                } else if kc == 'g' {
                    self.tank_change_gun_load_menu();
                    refresh = true;
                } else if load_ammo_menu || kc == 'm' {
                    load_ammo_menu = false;
                    self.main_gun_ammo_menu(true);
                    refresh = true;
                }
            }
        }
    }

    /// `MainGunAmmoMenu`: move shells between the supply trucks (when
    /// resupplying), general stores and the ready rack. Alt switched the
    /// direction of a move in the original; a terminal cannot see Alt on
    /// its own, so Tab does.
    pub fn main_gun_ammo_menu(&mut self, no_dark: bool) {
        if self.scr.is_headless() {
            self.autoload_ammo();
            return;
        }
        const KEY_CODES: [(char, char); 4] = [('U', 'J'), ('I', 'K'), ('O', 'L'), ('P', ';')];
        let mut inward = true;
        if !no_dark {
            self.darken_screen();
        }

        loop {
            let t = &self.tank;
            let camp = &self.campaign;
            let m = &mut self.menu_con;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Main Gun Ammunition Menu");

            let mut x = MENU_CON_XM - 28;
            m.fg = color::LIGHT_GREY;
            m.bg = color::DARKEST_GREY;
            m.print_ex(x + 2, 3, Align::Left, "Ammo");
            if camp.resupply {
                m.print_ex(x + 2, 5, Align::Left, "Supply");
                m.rect(x + 2, 5, 48, 2, false);
            }
            m.print_ex(x + 2, 9, Align::Left, "General");
            m.print_ex(x + 3, 10, Align::Left, "Stores");
            m.rect(x + 2, 9, 48, 2, false);
            m.print_ex(x + 2, 13, Align::Left, "Ready");
            m.print_ex(x + 3, 14, Align::Left, "Rack");
            m.rect(x + 2, 13, 48, 2, false);
            m.bg = color::BLACK;

            x += 16;
            let (mut total_g, mut total_rr) = (0, 0);
            for (key_code, a) in AMMO_TYPES.iter().filter(|a| t.general_ammo.has(a)).enumerate() {
                m.fg = color::LIGHT_GREY;
                m.print_ex(x, 3, Align::Right, a);
                m.fg = color::WHITE;
                if camp.resupply {
                    let text = match *a {
                        "HE" | "AP" | "WP" => "\u{ec}".to_string(),
                        "HCBI" if camp.hcbi > 0 => camp.hcbi.to_string(),
                        "HVAP" if camp.hvap > 0 => camp.hvap.to_string(),
                        "APDS" if camp.apds > 0 => camp.apds.to_string(),
                        _ => "-".to_string(),
                    };
                    m.print_ex(x, 5, Align::Right, &text);
                }
                m.print_ex(x, 9, Align::Right, &t.general_ammo.get(a).to_string());
                total_g += t.general_ammo.get(a);
                m.print_ex(x, 13, Align::Right, &t.rr_ammo.get(a).to_string());
                total_rr += t.rr_ammo.get(a);

                m.fg = color::LIGHT_GREY;
                let (k1, k2) = KEY_CODES[key_code];
                let (up, down) = ('\u{18}', '\u{19}');
                if camp.resupply {
                    let arrow = if inward { down } else { up };
                    m.print_ex(x, 7, Align::Right, &format!("{arrow}{k1}{arrow}"));
                    m.set_char_fg(x - 1, 7, KEY_HIGHLIGHT_COLOR);
                }
                let arrow = if inward { down } else { up };
                m.print_ex(x, 11, Align::Right, &format!("{arrow}{k2}{arrow}"));
                m.set_char_fg(x - 1, 11, KEY_HIGHLIGHT_COLOR);
                x += 8;
            }

            let max_x = MENU_CON_XM + 21;
            m.fg = color::LIGHT_GREY;
            m.print_ex(max_x, 3, Align::Right, "Max");
            m.fg = color::WHITE;
            m.print_ex(max_x, 9, Align::Right, &format!("{total_g}/{}", t.stats.int("main_gun_rounds")));
            m.print_ex(max_x, 13, Align::Right, &format!("{total_rr}/{}", t.stats.int("rr_size")));
            m.fg = color::LIGHT_GREY;
            m.frame(MENU_CON_XM - 28, 2, 52, 14, false, false);

            let lx = MENU_CON_XM - 46;
            for (n, line) in [
                "HE: High Explosive, used against infantry targets and in advancing fire",
                "AP: Armour-Piercing, used against armoured targets",
                "WP: White Phosporous, generates smoke and can pin infantry",
                "HCBI: Hexachlorothane-Base Initiating, generates a great deal of smoke but no other effect",
                "HVAP: High Velocity Armour-Piercing, used against armoured targets, more effective than AP",
                "APDS: Armour-Piercing Discarding Sabot, used against armoured targets, more effective than AP",
            ]
            .iter()
            .enumerate()
            {
                m.print_ex(lx, 17 + n as i32, Align::Left, line);
            }
            m.fg = color::WHITE;
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 6, Align::Center, "Use listed keys to move ammunition");
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 5, Align::Center, &format!("[{}] Switch Direction of Move", hl("Tab")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 4, Align::Center, &format!("[{}] Move 10 shells", hl("+Shift")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Close Menu and Continue", hl("Enter")));
            self.blit_menu();
            self.flush();

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                self.flush();
                let vk = self.key_vk();
                if vk == Vk::Enter {
                    return;
                }
                if vk == Vk::Tab {
                    inward = !inward;
                    refresh = true;
                    continue;
                }
                let kc = self.key_char();
                let cap = self.tank.stats.int("main_gun_rounds") + EXTRA_AMMO;
                let types: Vec<&str> = AMMO_TYPES.iter().copied().filter(|a| self.tank.general_ammo.has(a)).collect();
                if let Some(n) = ['j', 'k', 'l', ';'].iter().position(|&k| kc.to_ascii_lowercase() == k || (k == ';' && kc == ':')) {
                    let Some(&a) = types.get(n) else { continue };
                    let t = &mut self.tank;
                    if !inward {
                        if t.rr_ammo.get(a) > 0 && total_g < cap {
                            t.rr_ammo.add(a, -1);
                            t.general_ammo.add(a, 1);
                            self.play_sound("shell_move");
                        }
                    } else if t.general_ammo.get(a) > 0 && total_rr < t.stats.int("rr_size") {
                        t.general_ammo.add(a, -1);
                        t.rr_ammo.add(a, 1);
                        self.play_sound("shell_move");
                    }
                    refresh = true;
                } else if let Some(n) = ['u', 'i', 'o', 'p'].iter().position(|&k| kc.to_ascii_lowercase() == k) {
                    if !self.campaign.resupply {
                        continue;
                    }
                    let mut amount = if self.scr.key.shift { 10 } else { 1 };
                    let Some(&a) = types.get(n) else { continue };
                    if !inward {
                        if self.tank.general_ammo.get(a) >= amount {
                            self.tank.general_ammo.add(a, -amount);
                            match a {
                                "HCBI" => self.campaign.hcbi += amount,
                                "HVAP" => self.campaign.hvap += amount,
                                "APDS" => self.campaign.apds += amount,
                                _ => {}
                            }
                            self.play_sound("shell_move");
                        }
                    } else {
                        if total_g + amount > cap {
                            amount = cap - total_g;
                        }
                        let stock = match a {
                            "HCBI" => Some(&mut self.campaign.hcbi),
                            "HVAP" => Some(&mut self.campaign.hvap),
                            "APDS" => Some(&mut self.campaign.apds),
                            _ => None,
                        };
                        if let Some(s) = stock {
                            if *s < amount {
                                continue;
                            }
                            *s -= amount;
                        }
                        self.tank.general_ammo.add(a, amount);
                        self.play_sound("shell_move");
                    }
                    refresh = true;
                }
            }
        }
    }

    /// The ammunition menu for headless runs: with supplies on hand, fill
    /// general stores to the tank's capacity, about half AP and the rest
    /// HE with a few of each special round; keep the ready rack full.
    fn autoload_ammo(&mut self) {
        let cap = self.tank.stats.int("main_gun_rounds");
        let rr_size = self.tank.stats.int("rr_size");
        if self.campaign.resupply {
            let total: i32 = AMMO_TYPES.iter().map(|a| self.tank.general_ammo.get(a)).sum();
            let mut room = (cap - total).max(0);
            let mut wp = 4;
            for a in ["HVAP", "APDS", "HCBI", "WP"] {
                if !self.tank.general_ammo.has(a) {
                    continue;
                }
                let stock = match a {
                    "HCBI" => &mut self.campaign.hcbi,
                    "HVAP" => &mut self.campaign.hvap,
                    "APDS" => &mut self.campaign.apds,
                    _ => &mut wp,
                };
                let n = (*stock).min(room).min(4);
                *stock -= n;
                self.tank.general_ammo.add(a, n);
                room -= n;
            }
            let ap = room / 2;
            self.tank.general_ammo.add("AP", ap);
            self.tank.general_ammo.add("HE", room - ap);
        }
        let in_rr: i32 = AMMO_TYPES.iter().map(|a| self.tank.rr_ammo.get(a)).sum();
        let mut room = (rr_size - in_rr).max(0);
        while room > 0 {
            let Some(a) = ["AP", "HE"].into_iter().find(|a| self.tank.general_ammo.get(a) > 0) else { break };
            self.tank.general_ammo.add(a, -1);
            self.tank.rr_ammo.add(a, 1);
            room -= 1;
        }
    }

    /// `CampaignMenu`: the day's VP and how it ended; true if the player
    /// chose to save and quit to the main menu.
    pub fn campaign_menu(&mut self) -> bool {
        self.darken_screen();
        let m = &mut self.menu_con;
        m.clear();
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.fg = MENU_TITLE_COLOR;
        m.print_ex(MENU_CON_XM, 1, Align::Center, "Campaign Menu");
        m.fg = color::WHITE;
        let c = &self.campaign;
        if c.sunset {
            m.print_ex(MENU_CON_XM, 4, Align::Center, &format!("[{}] Return to Campaign Calendar", hl("ESC")));
        } else {
            m.print_ex(MENU_CON_XM, 4, Align::Center, &format!("[{}] Return to Game", hl("ESC")));
            m.print_ex(MENU_CON_XM, 5, Align::Center, &format!("[{}] Save Game, Return to Main Menu", hl("Q")));
        }
        m.print_ex(MENU_CON_XM, 8, Align::Center, &format!("VP Today: {}", c.day_vp));
        m.print_ex(MENU_CON_XM, 9, Align::Center, &format!("Campaign VP: {}", c.day_vp + c.vp));
        let t = &self.tank;
        let status = if !t.alive {
            "Your tank has been destroyed"
        } else if t.swiss_cheese {
            "Your tank has been damaged beyond repair"
        } else if t.immobilized {
            "Your tank has been immobilized"
        } else if c.sunset {
            "The combat day has ended"
        } else {
            ""
        };
        m.print_ex(MENU_CON_XM, 10, Align::Center, status);
        self.blit_menu();
        self.flush();

        loop {
            self.check_for_event();
            if self.key_vk() == Vk::Escape || self.scr.is_headless() {
                break;
            }
            if !self.campaign.sunset && self.tank.alive && self.key_char().eq_ignore_ascii_case(&'q') {
                return true;
            }
            self.flush();
        }
        if !self.campaign.sunset && self.tank.alive {
            self.render_campaign(false);
        }
        false
    }

    /// `InitCampaignDay`: the morning briefing, the drive to the start
    /// area, today's map, then the day itself.
    pub fn init_campaign_day(&mut self, load: bool) {
        if load {
            if self.campaign.selected_crew.is_some() {
                self.campaign.selected_crew = Some(0);
            }
            self.campaign.exiting = false;
            self.paint_campaign_map();
            self.update_c_action_con();
            self.con.clear();
        } else {
            self.start_campaign_day();
        }

        self.do_campaign_day();

        if self.campaign.sunset {
            let c = &mut self.campaign;
            c.vp += c.day_vp;
            if c.day_vp > c.record_day_vp {
                c.record_day_vp = c.day_vp;
            }
            c.day_vp = 0;
            c.day_in_progress = false;
        }
    }

    /// The new day's half of `InitCampaignDay`.
    fn start_campaign_day(&mut self) {
        self.campaign.add_stat("Days of Combat", 1);
        self.reset_for_new_day();
        self.campaign.day_in_progress = true;
        self.battle = None;
        let (h, m) = self.campaign.get_sunrise();
        self.campaign.hour = h;
        self.campaign.minute = m;

        self.campaign.resupply = true;
        self.generate_ammo();
        self.campaign_view_tank(true);
        self.campaign.resupply = false;
        self.campaign.clear_ammo();

        match self.campaign.scen_type.as_str() {
            "Advance" => self.tutorial_message("advance_mission"),
            "Battle" => self.tutorial_message("battle_mission"),
            "Counterattack" => self.tutorial_message("counterattack_mission"),
            _ => {}
        }

        let roll = self.roll_1d10();
        let hours_elapsed = roll / 2 + 1;
        let mut ammo_expended = roll * 2;
        let he = self.tank.general_ammo.get("HE");
        if he < ammo_expended {
            ammo_expended = he;
        }
        self.tank.general_ammo.add("HE", -ammo_expended);
        self.campaign.hour += hours_elapsed;
        self.update_date_con();

        let hs = if hours_elapsed > 1 { "s" } else { "" };
        let rs = if ammo_expended != 1 { "s" } else { "" };
        let text = format!(
            "Heading to your start area took {hours_elapsed} hour{hs} and your tank expended {ammo_expended} HE round{rs}."
        );
        log::info!("day: {text}");
        self.con.bg = color::BLACK;
        self.con.clear();
        self.con.print_ex(SCREEN_XM, SCREEN_HEIGHT / 2 - 2, Align::Center, &text);
        self.con.print_ex(SCREEN_XM, SCREEN_HEIGHT / 2 + 2, Align::Center, "Press Enter to continue");
        self.blit_con();
        self.flush();
        self.wait_for_enter();

        self.generating_map_screen();
        while !self.generate_campaign_map() {}
        self.update_c_action_con();
        let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
        self.update_c_info_con(mx, my);
        self.con.clear();

        self.tank_set_lead_tank();
        let p = self.campaign.day_map().player_node;
        self.move_view_to(Some(p));

        if self.campaign.scen_type == "Counterattack" {
            self.do_enemy_advance();
        } else {
            self.setup_check_area();
            self.campaign.free_check = true;
            self.popup("Select an adjacent area to check for enemy resistance");
            self.render_campaign(false);
        }
        self.save_game();
    }

    /// `DoCampaignDay`: the day's input loop, until sunset or the player
    /// quits.
    pub fn do_campaign_day(&mut self) {
        let p = self.campaign.day_map().player_node;
        self.move_view_to(Some(p));
        if self.battle.is_some() {
            // a saved game from the middle of a battle
            self.init_encounter(true, false);
            if self.campaign.exiting {
                return;
            }
            self.post_encounter(false);
        } else {
            self.update_date_con();
            self.update_c_action_con();
            self.update_c_overlay(None);
            self.render_campaign(false);
        }

        let mut steps = 0;
        loop {
            if self.campaign.exiting || self.campaign.sunset {
                return;
            }
            if self.scr.is_headless() {
                steps += 1;
                assert!(steps < 5000, "campaign day does not end");
                self.day_autopilot();
                continue;
            }

            self.check_for_event();
            let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
            if (mx, my) != self.campaign.mouseover {
                self.campaign.mouseover = (mx, my);
                self.update_c_info_con(mx, my);
                self.render_campaign(false);
            }

            let vk = self.key_vk();
            match vk {
                Vk::Escape => {
                    if self.campaign_menu() {
                        self.campaign.exiting = true;
                        self.save_game();
                        return;
                    }
                }
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
                _ => {}
            }

            let kc = self.key_char().to_ascii_lowercase();
            if kc == 'w' || vk == Vk::Up || kc == 's' || vk == Vk::Down || self.scr.mouse.wheel_up || self.scr.mouse.wheel_down {
                let step = if kc == 'w' || vk == Vk::Up || self.scr.mouse.wheel_up { -10 } else { 10 };
                self.campaign.c_map_y += step;
                self.campaign.check_y_offset(C_MAP_CON_HEIGHT, C_MAP_CON_WINDOW_H);
                self.update_c_info_con(mx, my);
                self.render_campaign(false);
            }

            match self.campaign.input_mode.as_str() {
                "Check Adjacent Area" => {
                    if vk == Vk::Tab {
                        self.select_next_area();
                        self.move_view_to(self.campaign.selected_node);
                    } else if vk == Vk::Enter {
                        self.move_view_to(self.campaign.selected_node);
                        self.check_area();
                        self.check_sunset();
                        self.random_campaign_event();
                    } else if vk == Vk::Backspace && !self.campaign.free_check {
                        self.cancel_action();
                    }
                }
                "Move Into Adjacent Area" => {
                    if vk == Vk::Tab {
                        self.select_next_area();
                        self.move_view_to(self.campaign.selected_node);
                    } else if vk == Vk::Enter {
                        self.move_view_to(self.campaign.selected_node);
                        self.move_area();
                        if self.campaign.exiting {
                            return;
                        }
                        self.check_sunset();
                        self.move_view_to(self.campaign.selected_node);
                    } else if vk == Vk::Backspace {
                        self.cancel_action();
                    }
                }
                "Call in Strike" => {
                    if vk == Vk::Tab {
                        self.select_next_area();
                        self.move_view_to(self.campaign.selected_node);
                    } else if kc == 'a' || kc == 'r' {
                        self.move_view_to(self.campaign.selected_node);
                        self.call_strike(kc);
                        self.check_sunset();
                        self.random_campaign_event();
                    } else if vk == Vk::Backspace {
                        self.cancel_action();
                    }
                }
                _ => self.day_action_key(kc),
            }
            self.flush();
        }
    }

    /// `CancelAction` in `DoCampaignDay`.
    fn cancel_action(&mut self) {
        self.campaign.input_mode = "None".into();
        self.update_c_action_con();
        self.update_c_overlay(None);
        self.render_campaign(false);
    }

    /// A key for a new action, with no action under way.
    fn day_action_key(&mut self, kc: char) {
        if self.campaign.scen_type != "Counterattack" {
            if kc == 'c' {
                self.setup_check_area();
                self.move_view_to(self.campaign.selected_node);
            } else if kc == 'a' {
                self.setup_call_strike();
                self.move_view_to(self.campaign.selected_node);
            }
        } else if kc == 'a' {
            self.await_enemy(false);
        }

        if kc == 'e' {
            self.setup_move_area();
            self.move_view_to(self.campaign.selected_node);
        } else if kc == 'r' {
            if self.popup_confirm("Spend 15 mins. trying to get resupplied?") {
                self.setup_resupply();
                self.check_sunset();
                if self.campaign.sunset {
                    return;
                }
                if self.campaign.scen_type == "Counterattack" {
                    self.await_enemy(true);
                }
            } else {
                self.render_campaign(false);
            }
        } else if kc == 'v' {
            self.campaign_view_tank(false);
            self.render_campaign(false);
        } else if kc == 'h' && crate::defs::defs().ending_damages.iter().any(|d| self.tank.has_damage(d)) {
            if self.popup_confirm("Return to HQ and allow the rest of your battlegroup to continue on alone?") {
                self.head_home();
            }
        }
    }

    /// Headless play for tests: on the way to the exit, check the next
    /// area now and then, call in strikes, resupply when short of shells;
    /// in a counterattack, mostly wait for the enemy.
    fn day_autopilot(&mut self) {
        match self.campaign.input_mode.as_str() {
            "Check Adjacent Area" => {
                // the free check, or one the autopilot set up
                let unknown = self.campaign.selected_node.is_some_and(|s| {
                    let n = &self.campaign.day_map().nodes[s];
                    !n.friendly_control && !n.res_known
                });
                if unknown {
                    self.check_area();
                    self.check_sunset();
                    self.random_campaign_event();
                }
                self.campaign.free_check = false;
                self.campaign.input_mode = "None".into();
                return;
            }
            "None" => {}
            _ => {
                self.campaign.input_mode = "None".into();
                return;
            }
        }

        let short_of_shells = self.tank.general_ammo.get("HE") + self.tank.general_ammo.get("AP") < 10;
        let roll = self.rng.int(1, 100);
        if self.campaign.scen_type == "Counterattack" {
            if short_of_shells || roll <= 5 {
                self.setup_resupply();
                self.check_sunset();
                if !self.campaign.sunset {
                    self.await_enemy(true);
                }
            } else if roll <= 15 {
                self.campaign.selected_node = None;
                self.select_next_area();
                if self.campaign.selected_node.is_some() {
                    self.move_area();
                    if !self.campaign.exiting {
                        self.check_sunset();
                    }
                } else {
                    self.await_enemy(false);
                }
            } else {
                self.await_enemy(false);
            }
            return;
        }

        if crate::defs::defs().ending_damages.iter().any(|d| self.tank.has_damage(d)) && roll <= 30 {
            self.head_home();
            return;
        }
        if short_of_shells {
            self.setup_resupply();
            return;
        }

        // the next area on the way to the exit
        let map = self.campaign.day_map();
        let player = map.player_node;
        let exit = map.nodes.iter().position(|n| n.exit);
        let next = exit.and_then(|e| self.get_path(player, e, false).first().copied()).or(map.nodes[player].links.first().copied());
        let Some(next) = next else {
            self.spend_time(1, 0);
            self.check_sunset();
            return;
        };
        let n = &self.campaign.day_map().nodes[next];
        let unknown = !n.friendly_control && !n.res_known;
        let struck = n.arty_strike || n.air_strike;
        let enemy = !n.friendly_control;
        self.campaign.selected_node = Some(next);
        if unknown && roll <= 40 {
            self.check_area();
            self.check_sunset();
            self.random_campaign_event();
        } else if enemy && !struck && roll <= 60 {
            let before = (self.campaign.hour, self.campaign.minute);
            self.call_strike(if roll <= 50 { 'a' } else { 'r' });
            // a strike the weather forbids spends no time; wait instead
            if (self.campaign.hour, self.campaign.minute) == before {
                self.spend_time(0, 15);
            }
            self.check_sunset();
            self.random_campaign_event();
        } else {
            self.move_area();
            if !self.campaign.exiting {
                self.check_sunset();
            }
        }
    }
}
