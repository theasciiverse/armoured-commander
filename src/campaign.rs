//! `Weather` and `Campaign` methods that reach the screen or the rest of
//! the game.

use crate::game::*;
use crate::layout::*;
use crate::model::*;

impl Game {
    /// `Weather.GenerateNew`: new weather on moving into a new area.
    pub fn weather_generate_new(&mut self) {
        self.campaign.weather = crate::model::Weather::default();
        let month = self.campaign.current_date[1];
        let (_, _, mut roll) = self.roll_2d6();
        if (3..=11).contains(&month) {
            roll -= 1;
        }
        if (5..=8).contains(&month) {
            roll -= 1;
        }
        if roll > 6 {
            self.campaign.weather.clouds = "Overcast".into();
        }
        if self.campaign.weather.clouds == "Overcast" {
            let (_, _, roll) = self.roll_2d6();
            if roll <= 4 {
                self.campaign.weather.precip = if month <= 2 || month == 12 {
                    "Snow".into()
                } else if (5..=9).contains(&month) {
                    "Rain".into()
                } else {
                    let (_, _, r) = self.roll_2d6();
                    if r >= 11 { "Snow".into() } else { "Rain".into() }
                };
            }
            let (_, _, mut roll) = self.roll_2d6();
            if self.campaign.weather.precip != "None" {
                roll -= 2;
            }
            if roll >= 10 {
                self.campaign.weather.fog = true;
            }
        }
        let (_, _, roll) = self.roll_2d6();
        let w = &mut self.campaign.weather;
        if w.precip == "Snow" {
            if roll >= 11 {
                w.ground = "Deep Snow".into();
            } else if roll >= 5 {
                w.ground = "Snow".into();
            }
        } else if w.precip == "Rain" {
            if roll >= 8 {
                w.ground = "Mud".into();
            }
        } else if month <= 2 || month == 12 {
            if roll == 12 {
                w.ground = "Deep Snow".into();
            } else if roll >= 8 {
                w.ground = "Snow".into();
            }
        } else if (5..=9).contains(&month) {
            if roll >= 10 {
                w.ground = "Mud".into();
            }
        } else if roll == 12 {
            w.ground = "Snow".into();
        } else if roll >= 8 {
            w.ground = "Mud".into();
        }
    }

    /// `Weather.CheckChange`
    pub fn weather_check_change(&mut self) {
        let (_, _, roll) = self.roll_2d6();
        let month = self.campaign.current_date[1];
        if self.campaign.weather.precip != "None" {
            if roll <= 3 {
                let text = if self.campaign.weather.precip == "Rain" { "The rain stops." } else { "The snow stops falling." };
                self.popup(text);
                self.campaign.weather.precip = "None".into();
            }
            return;
        } else if self.campaign.weather.clouds == "Overcast" {
            if roll <= 3 {
                let snow = if month <= 2 || month == 12 {
                    true
                } else if (5..=9).contains(&month) {
                    false
                } else {
                    let (_, _, r) = self.roll_2d6();
                    r >= 11
                };
                self.campaign.weather.precip = if snow { "Snow" } else { "Rain" }.into();
                self.popup(if snow { "Snow starts falling" } else { "Rain starts falling" });
                return;
            }
        }
        let (_, _, roll) = self.roll_2d6();
        if self.campaign.weather.clouds == "Clear" {
            if roll <= 3 {
                self.campaign.weather.clouds = "Overcast".into();
                self.popup("Clouds roll in and the weather turns overcast");
            }
        } else {
            if roll <= 5 {
                if self.campaign.weather.fog {
                    self.campaign.weather.fog = false;
                    self.popup("The fog lifts.");
                    return;
                }
                self.campaign.weather.clouds = "Clear".into();
                self.campaign.weather.precip = "None".into();
                self.popup("The sky clears.");
                return;
            }
            let (_, _, roll) = self.roll_2d6();
            if roll <= 3 && !self.campaign.weather.fog {
                self.campaign.weather.fog = true;
                self.popup("Fog rolls in.");
            }
        }
    }

    /// `Weather.CheckGround`: rain, snow and dry spells change the ground.
    pub fn weather_check_ground(&mut self, minutes_passed: i32) {
        let mut change = false;
        let precip = self.campaign.weather.precip.clone();
        let ground = self.campaign.weather.ground.clone();
        if precip == "Rain" {
            if ground != "Mud" {
                self.campaign.weather.rain_time += minutes_passed;
                if self.campaign.weather.rain_time >= 120 {
                    self.popup("The rain has turned to ground to mud.");
                    let w = &mut self.campaign.weather;
                    w.ground = "Mud".into();
                    w.dry_time = 0;
                    change = true;
                }
            }
        } else if precip == "Snow" {
            if ground != "Deep Snow" {
                self.campaign.weather.snow_time += minutes_passed;
                if self.campaign.weather.snow_time >= 120 {
                    if ground == "Snow" {
                        self.popup("The snow on the ground has become deep.");
                        self.campaign.weather.ground = "Deep Snow".into();
                        change = true;
                    } else if ground == "Dry" || ground == "Mud" {
                        self.popup("The ground is covered in snow.");
                        self.campaign.weather.ground = "Snow".into();
                        change = true;
                    }
                }
            }
        } else if ground == "Mud" {
            self.campaign.weather.dry_time += minutes_passed;
            if self.campaign.weather.dry_time >= 120 {
                self.popup("The muddy ground dries out.");
                let w = &mut self.campaign.weather;
                w.ground = "Dry".into();
                w.rain_time = 0;
                change = true;
            }
        }
        if change {
            if self.battle.is_some() {
                self.update_map_overlay(false);
            } else {
                self.build_action_list();
                self.update_c_action_con();
                let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
                self.update_c_info_con(mx, my);
            }
        }
    }

    /// `SpendTime`: move the clock on, with the weather and quest
    /// deadlines that follow.
    pub fn spend_time(&mut self, hours: i32, minutes: i32) {
        let c = &mut self.campaign;
        c.minute += minutes;
        while c.minute >= 60 {
            c.hour += 1;
            c.minute -= 60;
        }
        c.hour += hours;
        self.update_date_con();
        self.weather_check_ground(hours * 60 + minutes);

        let checks = hours * 4 + (minutes as f64 / 15.0).ceil() as i32;
        for _ in 0..checks {
            if self.rng.int(1, 10) == 1 {
                self.weather_check_change();
                if self.battle.is_some() {
                    self.paint_map_con();
                } else {
                    self.build_action_list();
                    self.update_c_action_con();
                    let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
                    self.update_c_info_con(mx, my);
                }
            }
        }

        if self.campaign.quest_active {
            let (hour, minute) = (self.campaign.hour, self.campaign.minute);
            let Some(map) = &self.campaign.day_map else { return };
            if let Some(n) = map.nodes.iter().position(|n| n.quest_time_limit.is_some()) {
                let (h, m) = map.nodes[n].quest_time_limit.unwrap_or((0, 0));
                if hour > h || (hour == h && minute > m) {
                    let quest = map.nodes[n].quest_type.clone().unwrap_or_default();
                    let text = format!("Time has run out to complete {quest} quest.");
                    self.popup(&text);
                    self.write_journal(&text);
                    let node = &mut self.campaign.day_map_mut().nodes[n];
                    node.quest_type = None;
                    node.quest_vp_bonus = None;
                    node.quest_time_limit = None;
                    self.campaign.quest_active = false;
                    if self.battle.is_none() {
                        self.update_c_overlay(None);
                    }
                }
            }
        }
    }

    /// `BuildActionList`: what the player can do on the campaign map, and
    /// how long each takes, in words shortened to fit the 39-column panel.
    pub fn build_action_list(&mut self) {
        let w = &self.campaign.weather;
        let ti = if w.ground != "Dry" || w.precip != "None" || w.fog { GROUND_MOVE_TIME_MODIFIER } else { 0 };
        let air_ok = w.clouds != "Overcast" && !w.fog && w.precip != "Snow";
        let mut list: Vec<(String, Option<i32>)> = Vec::new();
        if self.campaign.scen_type == "Counterattack" {
            list.push((format!("[{}]wait Enemy Counterattack", hl("A")), None));
            list.push((format!("[{}]nter friendly, Improved Road", hl("E")), Some(STONE_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter friendly, Dirt Road", hl("E")), Some(DIRT_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter friendly, No Road", hl("E")), Some(NO_ROAD_MOVE_TIME + ti)));
            list.push((format!("Attempt [{}]esupply", hl("R")), Some(15)));
        } else {
            list.push((format!("[{}]heck adjacent area", hl("C")), Some(15)));
            list.push((format!("[{}]nter area, Improved Road", hl("E")), Some(STONE_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter area, Dirt Road", hl("E")), Some(DIRT_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter area, No Road", hl("E")), Some(NO_ROAD_MOVE_TIME + ti)));
            list.push((format!("Call [{}]rtillery or Air Strike", hl("A")), Some(15)));
            if air_ok {
                list.push((format!("Call [{}]ir Strike on adjacent area", hl("A")), Some(30)));
            }
            list.push((format!("Attempt [{}]esupply", hl("R")), Some(60)));
        }
        list.push((format!("[{}]iew Tank", hl("V")), None));
        let ending = defs_ending_damage(&self.tank.damage_list);
        if ending {
            list.push((format!("Return to [{}]Q", hl("H")), None));
        }
        self.campaign.action_list = list;
    }

    /// `ResetForNewDay`: what starts afresh on each day of action.
    pub fn reset_for_new_day(&mut self) {
        let c = &mut self.campaign;
        c.day_map = None;
        self.weather_generate_new();
        let c = &mut self.campaign;
        c.hour = 0;
        c.minute = 0;
        c.c_map_y = 0;
        c.selected_crew = None;
        c.resupply = false;
        c.input_mode = "None".into();
        c.selected_node = None;
        c.adjacent_nodes.clear();
        c.free_check = false;
        c.messages.clear();
        c.sunset = false;
        c.exiting = false;
        c.arty_chance = 9;
        c.air_chance = 7;
        c.time_of_last_event = (0, 0);
        c.quest_active = false;
        self.build_action_list();
    }

    /// `DoEnemyAdvance`: in a counterattack, the enemy takes friendly areas
    /// on the front line; if that cuts the player off from the exit, they
    /// fall back to the nearest friendly area.
    pub fn do_enemy_advance(&mut self) {
        let map = self.campaign.day_map();
        let mut nodes = Vec::new();
        for (i, node) in map.nodes.iter().enumerate() {
            if i == map.player_node || !node.friendly_control || map.blocked_nodes.contains(&i) {
                continue;
            }
            if node.top_edge || node.links.iter().any(|&l| !map.nodes[l].friendly_control) {
                nodes.push(i);
            }
        }
        if nodes.is_empty() {
            return;
        }

        for i in nodes {
            let map = self.campaign.day_map();
            let mut chance = 0;
            for &l in &map.nodes[i].links {
                let ln = &map.nodes[l];
                if !ln.friendly_control {
                    match ln.resistance.as_deref() {
                        Some("Light") if chance < 3 => chance = 3,
                        Some("Medium") if chance < 5 => chance = 5,
                        Some("Heavy") if chance < 7 => chance = 7,
                        _ => {}
                    }
                }
            }
            // at the start of the day the top edge has no enemy beside it
            if chance == 0 {
                chance = *self.rng.choice(&[3, 5, 7]);
            }
            let (_, _, roll) = self.roll_2d6();
            if roll <= chance {
                let n = &mut self.campaign.day_map_mut().nodes[i];
                n.friendly_control = false;
                n.res_known = true;
                self.move_view_to(Some(i));
                self.update_c_overlay(Some(i));
                self.render_campaign(false);
                self.wait(500);
                self.popup("A map area has been captured by an enemy advance.");
                self.update_c_overlay(None);
                self.render_campaign(false);
            }
        }

        // still a way through to the exit?
        let map = self.campaign.day_map();
        let player = map.player_node;
        if !map.nodes[player].exit {
            if let Some(exit) = map.nodes.iter().position(|n| n.exit) {
                if !self.get_path(player, exit, true).is_empty() {
                    return;
                }
            }
        }

        let mut ammo_expended = self.roll_1d10() * 2;
        let he = self.tank.general_ammo.get("HE");
        if he < ammo_expended {
            ammo_expended = he;
        }
        self.tank.general_ammo.add("HE", -ammo_expended);

        let w = &self.campaign.weather;
        let mut time_req = 60;
        if w.ground != "Dry" || w.precip != "None" || w.fog {
            time_req += 15;
        }
        self.spend_time(0, time_req);
        self.popup(&format!(
            "You have been cut off from your allies and must reposition. You travel off-road to the nearest friendly \
             map area, expending {ammo_expended} HE shells to cover your withdrawl. This takes {time_req} minutes."
        ));

        let map = self.campaign.day_map_mut();
        let player = map.player_node;
        map.nodes[player].friendly_control = false;
        map.nodes[player].res_known = true;
        let (px, py) = (map.nodes[player].x, map.nodes[player].y);
        let mut closest = None;
        let mut closest_dist = 9000;
        for (i, n) in map.nodes.iter().enumerate() {
            if n.friendly_control {
                let d = get_distance(px, py, n.x, n.y);
                if d < closest_dist {
                    closest = Some(i);
                    closest_dist = d;
                }
            }
        }
        let Some(closest) = closest else {
            log::error!("could not find a friendly node to move to");
            return;
        };
        map.player_node = closest;
        self.move_view_to(Some(closest));
        self.update_c_overlay(None);
        self.render_campaign(false);
        self.wait(500);
        self.check_sunset();
    }

    /// `ShowNode` in `RandomCampaignEvent`: bring an area into view and
    /// highlight it.
    fn show_node(&mut self, i: usize) {
        self.move_view_to(Some(i));
        self.update_c_overlay(Some(i));
        self.render_campaign(false);
        self.wait(1000);
    }

    /// `RandomCampaignEvent`: at most one an hour, more likely the longer
    /// since the last: a supply truck, a quest, a new exit, news of an
    /// area, or an area changing hands.
    pub fn random_campaign_event(&mut self) {
        if self.campaign.scen_type == "Counterattack" || self.campaign.sunset {
            return;
        }
        let mut roll = self.roll_1d100();
        if self.campaign.time_of_last_event == (0, 0) {
            self.campaign.time_of_last_event = (self.campaign.hour, self.campaign.minute);
            return;
        }
        let (h1, m1) = self.campaign.time_of_last_event;
        let (h, m) = get_time_until(h1, m1, self.campaign.hour, self.campaign.minute);
        if h == 0 {
            return;
        } else if h == 1 {
            if m <= 15 {
                roll -= 30;
            } else if m <= 30 {
                roll -= 25;
            } else if m <= 45 {
                roll -= 10;
            }
        }

        if roll <= 50 {
            return;
        }
        let player = self.campaign.day_map().player_node;

        if roll <= 55 {
            self.write_journal("Friendly supply truck discovered.");
            if self.popup_confirm("You have encountered a friendly supply truck. Restock your ammunition? (15 mins.)") {
                self.spend_time(0, 15);
                self.tank.smoke_grenades = 6;
                self.tank.smoke_bombs = 15;
                self.campaign.resupply = true;
                self.main_gun_ammo_menu(false);
                self.campaign.resupply = false;
                self.render_campaign(false);
            }
            self.campaign.time_of_last_event = (self.campaign.hour, self.campaign.minute);
            return;
        }

        if roll <= 75 {
            if self.campaign.quest_active {
                return;
            }
            let (_, _, roll) = self.roll_2d6();
            let (quest_type, vp_bonus) = if roll <= 3 {
                ("RESCUE", 15)
            } else if roll <= 7 {
                ("CAPTURE", 10)
            } else if roll <= 10 {
                ("RECON", 5)
            } else {
                ("DEFEND", 15)
            };
            let map = self.campaign.day_map();
            let player_y = map.nodes[player].y;
            let mut nodes = Vec::new();
            for (i, node) in map.nodes.iter().enumerate() {
                if map.blocked_nodes.contains(&i) || node.node_type == "E" || i == player || node.y > player_y {
                    continue;
                }
                if (quest_type != "DEFEND") == node.friendly_control {
                    continue;
                }
                if quest_type == "RECON" && node.res_known {
                    continue;
                }
                if self.get_path(player, i, false).len() > 3 {
                    continue;
                }
                nodes.push(i);
            }
            if nodes.is_empty() {
                return;
            }
            self.write_journal(&format!("{quest_type} quest triggered"));
            self.campaign.quest_active = true;
            self.campaign.add_stat("Quests Assigned", 1);
            let node = *self.rng.choice(&nodes);
            let n = &mut self.campaign.day_map_mut().nodes[node];
            n.quest_type = Some(quest_type.into());
            n.quest_vp_bonus = Some(vp_bonus);
            let text = match quest_type {
                "RESCUE" => {
                    let h = self.campaign.hour + self.rng.int(2, 4);
                    let m = self.campaign.minute;
                    self.campaign.day_map_mut().nodes[node].quest_time_limit = Some((h, m));
                    format!(
                        "Commander, you are requested to head to the highlighted map location. Allied units are pinned \
                         down in the area and require your help. If completed at or before {h}:{m:02}, you will \
                         receive a bonus of {vp_bonus} VP."
                    )
                }
                "RECON" => format!(
                    "Commander, you are requested to head to the highlighted map location and check it for estimated \
                     enemy resistance. If completed, you will receive a bonus of {vp_bonus} VP."
                ),
                "CAPTURE" => format!(
                    "Commander, you are requested to head to the highlighted map location and capture it from enemy \
                     forces. If completed, you will receive a bonus of {vp_bonus} VP."
                ),
                _ => format!(
                    "Commander, you are requested to head to the highlighted map location and defend it from an \
                     anticipated enemy  counterattack. If completed, you will receive a bonus of {vp_bonus} VP."
                ),
            };
            self.show_node(node);
            self.popup(&text);
        } else if roll <= 80 {
            // a new exit, unless the player is nearly at the old one
            let map = self.campaign.day_map();
            if let Some(exit) = map.nodes.iter().position(|n| n.exit) {
                if self.get_path(player, exit, false).len() <= 4 {
                    return;
                }
            }
            let mut nodes = Vec::new();
            let mut old_exit = None;
            for (i, node) in map.nodes.iter().enumerate() {
                if !node.top_edge || node.friendly_control {
                    continue;
                }
                if node.exit {
                    old_exit = Some(i);
                } else if !self.get_path(player, i, false).is_empty() {
                    nodes.push(i);
                }
            }
            if nodes.is_empty() {
                return;
            }
            let node = *self.rng.choice(&nodes);
            let map = self.campaign.day_map_mut();
            map.nodes[node].exit = true;
            if let Some(o) = old_exit {
                map.nodes[o].exit = false;
            }
            self.show_node(node);
            self.popup("HQ has ordered us to proceed to a different target area.");
        } else if roll <= 85 {
            let map = self.campaign.day_map();
            let nodes: Vec<usize> = map.nodes[player]
                .links
                .iter()
                .copied()
                .filter(|&l| {
                    let n = &map.nodes[l];
                    !n.res_known && !n.friendly_control && n.quest_type.is_none()
                })
                .collect();
            if nodes.is_empty() {
                return;
            }
            let node = *self.rng.choice(&nodes);
            self.campaign.day_map_mut().nodes[node].res_known = true;
            self.show_node(node);
            self.popup("Reconnaissance teams have reported on a nearby area.");
        } else if roll <= 90 {
            let map = self.campaign.day_map();
            let nodes: Vec<usize> = map.nodes[player]
                .links
                .iter()
                .copied()
                .filter(|&l| {
                    let n = &map.nodes[l];
                    n.res_known && !n.friendly_control && n.resistance.as_deref() != Some("Heavy")
                })
                .collect();
            if nodes.is_empty() {
                return;
            }
            let node = *self.rng.choice(&nodes);
            let n = &mut self.campaign.day_map_mut().nodes[node];
            n.resistance = Some(if n.resistance.as_deref() == Some("Light") { "Medium" } else { "Heavy" }.into());
            self.show_node(node);
            self.popup("We have received reports of enemy reinforcement in a nearby area.");
        } else if roll <= 95 {
            let map = self.campaign.day_map();
            let nodes: Vec<usize> = (0..map.nodes.len())
                .filter(|&i| map.nodes[i].friendly_control && i != player && map.nodes[i].quest_type.is_none())
                .collect();
            if nodes.is_empty() {
                return;
            }
            let node = *self.rng.choice(&nodes);
            self.campaign.day_map_mut().nodes[node].friendly_control = false;
            self.show_node(node);
            self.popup("A map area has been recaptured by an enemy advance.");
        } else {
            let map = self.campaign.day_map();
            let nodes: Vec<usize> = map.nodes[player]
                .links
                .iter()
                .copied()
                .filter(|&l| {
                    let n = &map.nodes[l];
                    !n.exit && !n.friendly_control && l != player && n.quest_type.is_none()
                })
                .collect();
            if nodes.is_empty() {
                return;
            }
            let node = *self.rng.choice(&nodes);
            self.campaign.day_map_mut().nodes[node].friendly_control = true;
            self.show_node(node);
            self.popup("A nearby area has been captured by friendly forces.");
        }

        self.update_c_overlay(None);
        self.move_view_to(Some(player));
        self.render_campaign(false);
        self.campaign.time_of_last_event = (self.campaign.hour, self.campaign.minute);
    }

    /// `MoveViewTo`: scroll the day map, ten rows at a time, until the
    /// area is well inside the window.
    pub fn move_view_to(&mut self, node: Option<usize>) {
        let Some(i) = node else { return };
        let map = self.campaign.day_map();
        let (x, y, map_w, map_h) = (map.nodes[i].x, map.nodes[i].y, map.w, map.h);
        let c = &mut self.campaign;
        while c.c_map_y > y - 10 {
            c.c_map_y -= 10;
        }
        while c.c_map_y < y - c_map_window_h() + 10 {
            c.c_map_y += 10;
        }
        c.check_y_offset(map_h, c_map_window_h());
        // the same across, when the map is wider than the window
        while c.c_map_x > x - 10 {
            c.c_map_x -= 10;
        }
        while c.c_map_x < x - c_map_window_w() + 10 {
            c.c_map_x += 10;
        }
        c.check_x_offset(map_w, c_map_window_w());
        self.render_campaign(false);
    }

    /// `AwardCaptureVP`: VP for taking (or in a counterattack, holding) an
    /// area: more for villages, bocage and woods, 20 more for the exit,
    /// double on an advance.
    pub fn award_capture_vp(&mut self, node: usize, counterattack: bool) {
        let n = &self.campaign.day_map().nodes[node];
        let mut vp = match n.node_type.as_str() {
            "A" | "B" => 1,
            "C" | "F" => 3,
            _ => 2,
        };
        if n.exit {
            vp += 20;
        } else if self.campaign.scen_type == "Advance" {
            vp *= 2;
        }
        self.campaign.day_vp += vp;
        let what = if counterattack { "defending" } else { "capturing" };
        self.popup(&format!("You are awarded {vp} VP for {what} this area."));
    }

    /// `GenerateAmmo`: the rare shells on offer at the morning briefing.
    pub fn generate_ammo(&mut self) {
        self.campaign.hcbi = self.rng.int(1, 10);
        self.campaign.hvap = self.rng.int(1, 3);
        let (min_apds, max_apds) = if self.campaign.current_date[0] == 1945 { (2, 5) } else { (1, 3) };
        self.campaign.apds = self.rng.int(min_apds, max_apds);
        if self.pos_skill_check("Loader", "Scrounger") {
            self.campaign.hcbi += self.rng.int(1, 3);
            self.campaign.hvap += 1;
            self.campaign.apds += self.rng.int(1, 2);
        }
    }

    /// `CheckSunset`: end the day at sunset.
    pub fn check_sunset(&mut self) {
        if self.campaign.sunset {
            return;
        }
        let (h, m) = self.campaign.get_sunset();
        if self.campaign.hour > h || (self.campaign.hour == h && self.campaign.minute >= m) {
            self.popup("The sun has set and this day of combat is over.");
            self.write_journal(&format!("The sun set at {h}:{m:02} and the action day ended"));
            self.campaign.sunset = true;
            self.render_campaign(false);
            self.end_of_day();
        }
    }

    /// `HeadHome`: the commander takes a damaged tank back to HQ.
    pub fn head_home(&mut self) {
        self.render_campaign(false);
        self.popup("You have chosen to head back to HQ, ending your combat day.");
        let text = format!(
            "The commander chose to return to HQ at {}:{} because of tank damage",
            self.campaign.hour, self.campaign.minute
        );
        self.write_journal(&text);
        self.campaign.sunset = true;
        self.render_campaign(false);
        self.end_of_day();
    }

    /// `EndOfDay`: the day's experience, levels, and the campaign menu.
    pub fn end_of_day(&mut self) {
        for i in 0..self.tank.crew.len() {
            let (_, _, roll) = self.roll_2d6();
            self.crewm(i).award_exp(roll);
        }
        self.check_crew_levels();
        self.campaign_menu();
    }

    /// `CheckCrewLevels`
    pub fn check_crew_levels(&mut self) {
        for i in 0..self.tank.crew.len() {
            while self.crew(i).level < crate::defs::defs().level_cap {
                if self.crew(i).exp >= Game::get_exp_req(self.crew(i).level + 1) {
                    let c = self.crewm(i);
                    c.level += 1;
                    c.skill_pts += 1;
                    let text = format!("{} gained a level, is now level {}", c.name, c.level);
                    self.popup(&text);
                    self.write_journal(&text);
                } else {
                    break;
                }
            }
        }
    }
}

/// Damage that lets the commander head back to HQ (`ENDING_DAMAGES`).
fn defs_ending_damage(damage_list: &[String]) -> bool {
    crate::defs::defs().ending_damages.iter().any(|d| damage_list.contains(d))
}
