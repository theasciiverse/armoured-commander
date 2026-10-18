//! `Weather` and `Campaign` methods that reach the screen or the rest of
//! the game.

use crate::game::*;

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
    /// how long each takes.
    pub fn build_action_list(&mut self) {
        let w = &self.campaign.weather;
        let ti = if w.ground != "Dry" || w.precip != "None" || w.fog { GROUND_MOVE_TIME_MODIFIER } else { 0 };
        let air_ok = w.clouds != "Overcast" && !w.fog && w.precip != "Snow";
        let mut list: Vec<(String, Option<i32>)> = Vec::new();
        if self.campaign.scen_type == "Counterattack" {
            list.push((format!("[{}]wait Enemy Counterattack", hl("A")), None));
            list.push((format!("[{}]nter adjacent friendly area, Improved Road", hl("E")), Some(STONE_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter adjacent friendly area, Dirt Road", hl("E")), Some(DIRT_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter adjacent friendly area, No Road", hl("E")), Some(NO_ROAD_MOVE_TIME + ti)));
            list.push((format!("Attempt [{}]esupply", hl("R")), Some(15)));
        } else {
            list.push((format!("[{}]heck adjacent area", hl("C")), Some(15)));
            list.push((format!("[{}]nter adjacent area, Improved Road", hl("E")), Some(STONE_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter adjacent area, Dirt Road", hl("E")), Some(DIRT_ROAD_MOVE_TIME + ti)));
            list.push((format!("[{}]nter adjacent area, No Road", hl("E")), Some(NO_ROAD_MOVE_TIME + ti)));
            list.push((format!("Call for [{}]rtillery or Air Strike", hl("A")), Some(15)));
            if air_ok {
                list.push((format!("Call for [{}]ir Strike on adjacent area", hl("A")), Some(30)));
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
