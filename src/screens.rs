//! The information windows: help, tank info, crew info and skills,
//! settings, campaign stats and the journal.

use crate::color;
use crate::console::{Align, Console};
use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::ui::{Vk, wrap};

impl Game {
    /// `ShowHelp`: help topics in alphabetical order, the selected one
    /// in the middle; a letter jumps to the first topic starting with it.
    pub fn show_help(&mut self) {
        let mut help_list = defs::defs().help_text.clone();
        help_list.sort_by(|a, b| a.0.cmp(&b.0));
        let mut selected = 0usize;
        let x = 24;
        self.darken_screen();
        loop {
            let m = &mut self.menu_con;
            m.clear();
            m.align = Align::Left;
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Help Topics");
            m.fg = color::WHITE;
            m.print_ex(MENU_CON_XM, 2, Align::Center, "Type a letter to jump to its entry, arrow keys to scroll");
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Return", hl("ESC")));
            m.hline(1, 3, MENU_CON_WIDTH - 2);
            m.hline(1, MENU_CON_HEIGHT - 3, MENU_CON_WIDTH - 2);
            m.vline(x, 4, MENU_CON_HEIGHT - 7);

            let yc = MENU_CON_HEIGHT / 2;
            for (n, (topic, _)) in help_list.iter().enumerate() {
                let y = yc - (selected as i32 - n as i32);
                if y < 5 || y > MENU_CON_HEIGHT - 5 {
                    continue;
                }
                if n == selected {
                    m.bg = SELECTED_COLOR;
                    m.rect(2, y, x - 3, 1, false);
                    m.bg = color::BLACK;
                }
                m.print(2, y, topic);
            }
            let lines = wrap(&help_list[selected].1, (MENU_CON_WIDTH - x - 6) as usize, " ");
            let mut y = yc - lines.len() as i32 / 2;
            for line in &lines {
                m.print(x + 3, y, line);
                y += 1;
            }
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                let vk = self.key_vk();
                let kc = self.key_char();
                if vk == Vk::Escape {
                    return self.after_window();
                } else if vk == Vk::Up && selected > 0 {
                    selected -= 1;
                    refresh = true;
                } else if vk == Vk::Down && selected + 1 < help_list.len() {
                    selected += 1;
                    refresh = true;
                } else if kc.is_ascii_alphabetic() {
                    let c = kc.to_ascii_lowercase();
                    if let Some(n) = help_list.iter().position(|(t, _)| t.chars().next().is_some_and(|f| f.to_ascii_lowercase() == c)) {
                        selected = n;
                        refresh = true;
                    }
                }
                self.flush();
            }
        }
        self.after_window();
    }

    /// Redraw what was behind a window, as the original does on leaving
    /// one.
    fn after_window(&mut self) {
        if self.battle.is_some() {
            self.render_encounter(false);
        } else if self.campaign.day_in_progress {
            self.render_campaign(false);
        }
    }

    /// `ShowTankInfo`: the player's tank; with `select_tank`, scroll
    /// through the models available this month and return the choice.
    pub fn show_tank_info(&mut self, select_tank: bool) -> Option<String> {
        let tank_list: Vec<String> = if select_tank {
            self.campaign.player_veh_list.iter().filter(|v| self.campaign.get_rf(v) > 0).cloned().collect()
        } else {
            Vec::new()
        };
        let mut selected = 0usize;
        self.darken_screen();
        loop {
            let show = if select_tank { tank_list[selected].clone() } else { self.tank.unit_type.clone() };
            let m = &mut self.menu_con;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, if select_tank { "Select a Tank Model" } else { "Player Tank Info" });
            m.fg = color::WHITE;
            Game::show_vehicle_type_info(m, &show, 16, 4, false);
            let text = if select_tank {
                format!("[{}] Scroll [{}] Select Tank", hl("A/D/Left/Right"), hl("Enter"))
            } else {
                format!("[{}] Return", hl("ESC"))
            };
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &text);
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                let vk = self.key_vk();
                if vk == Vk::Escape && !select_tank || vk == Vk::Enter && select_tank {
                    self.after_window();
                    return select_tank.then(|| tank_list[selected].clone());
                }
                if select_tank {
                    let kc = self.key_char().to_ascii_lowercase();
                    if kc == 'a' || vk == Vk::Left {
                        selected = if selected > 0 { selected - 1 } else { tank_list.len() - 1 };
                        refresh = true;
                    } else if kc == 'd' || vk == Vk::Right {
                        selected = if selected + 1 < tank_list.len() { selected + 1 } else { 0 };
                        refresh = true;
                    }
                }
                self.flush();
            }
        }
        self.after_window();
        select_tank.then(|| tank_list[selected].clone())
    }

    /// `ShowSettings`. The original's full screen and resolution options
    /// have no terminal equivalent and are left out.
    pub fn show_settings(&mut self) {
        self.darken_screen();
        let x = MENU_CON_XM - 29;
        loop {
            let c = &self.campaign;
            let on = |b: bool| if b { "On" } else { "Off" };
            let m = &mut self.menu_con;
            m.clear();
            m.align = Align::Left;
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 2, Align::Center, "Game Settings Menu");
            m.fg = color::WHITE;
            m.print_ex(MENU_CON_XM, 3, Align::Center, &format!("{VERSION}{SUBVERSION}"));

            m.frame(x, 5, 40, 6, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print(x + 2, 6, "Campaign Settings");
            m.fg = color::WHITE;
            let tanks = if c.unlimited_tank_selection { "Unlimited" } else { "Strict" };
            m.print(x + 2, 8, &format!("Tank Selection: {tanks}"));
            let cmdr = if c.casual_commander { "Casual" } else { "Realistic" };
            m.print(x + 2, 9, &format!("Commander Replacement: {cmdr}"));

            m.frame(x, 12, 58, 8, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print(x + 2, 13, "Display Settings");
            m.fg = color::WHITE;
            m.print(x + 2, 15, &format!("[{}]nimations: {}", hl("A"), on(c.animations)));
            m.print(x + 2, 16, &format!("[{}]ounds: {}", hl("S"), on(c.sounds)));
            m.print(x + 2, 17, &format!("[{}]isplay Tutorial Messages: {}", hl("D"), on(c.tutorial_message)));
            m.print(x + 2, 18, &format!("[{}]ait for Enter before clearing on-screen labels: {}", hl("W"), on(c.pause_labels)));

            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 3, Align::Center, "Press highlighted letter to change setting");
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Return", hl("ESC")));
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                if self.key_vk() == Vk::Escape {
                    return self.after_window();
                }
                let c = &mut self.campaign;
                match self.scr.key.c.to_ascii_lowercase() {
                    'a' => c.animations = !c.animations,
                    's' => c.sounds = !c.sounds,
                    'w' => c.pause_labels = !c.pause_labels,
                    'd' => c.tutorial_message = !c.tutorial_message,
                    _ => {
                        self.flush();
                        continue;
                    }
                }
                log::info!("settings: animations={} sounds={} tutorial={} pause_labels={}", c.animations, c.sounds, c.tutorial_message, c.pause_labels);
                refresh = true;
            }
        }
        self.after_window();
    }

    /// `ShowCampaignStats`: days fought, VP, the campaign's tallies, and
    /// J for the journal.
    pub fn show_campaign_stats(&mut self) {
        self.darken_screen();
        loop {
            let c = &self.campaign;
            let stat = |name: &str| c.stats.iter().find(|(k, _)| k == name).map_or(0, |(_, v)| *v);
            let m = &mut self.menu_con;
            m.clear();
            m.align = Align::Left;
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 2, Align::Center, "Campaign Stats");
            m.fg = color::WHITE;

            m.bg = ROW_COLOR;
            m.rect(MENU_CON_XM - 20, 5, 40, 1, false);
            let d1 = stat("Days of Combat");
            let d2 = (c.days.len() as i32 - c.start_date as i32).max(1);
            m.print_ex(MENU_CON_XM, 5, Align::Center, &format!("Campaign Day {d1}/{d2}"));
            let w = (d1 as f64 / d2 as f64 * 40.0).ceil() as i32;
            m.bg = HIGHLIGHT_COLOR;
            m.rect(MENU_CON_XM - 20, 5, w, 1, false);
            m.bg = color::BLACK;

            let x = MENU_CON_XM - 20;
            m.print(x, 7, &format!("Total Victory Points: {}", c.vp + c.day_vp));
            let mut y = 9;
            for name in &defs::defs().c_stats {
                m.print(x, y, &format!("{name}: {}", stat(name)));
                y += 1;
            }
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 4, Align::Center, &format!("Display Campaign [{}]ournal", hl("J")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Return", hl("ESC")));
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                if self.key_vk() == Vk::Escape {
                    return self.after_window();
                }
                if self.key_char().eq_ignore_ascii_case(&'j') {
                    let journal = self.campaign.campaign_journal.clone();
                    self.show_text_window("Campaign Journal", &journal);
                    refresh = true;
                }
                self.flush();
            }
        }
        self.after_window();
    }

    /// `ShowTextWindow`: scroll through lines of text, starting at the
    /// end. The window shows 22 lines where the original showed 49.
    pub fn show_text_window(&mut self, title: &str, text_lines: &[String]) {
        Console::blit(&self.root, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, &mut self.con, 0, 0);
        let width = (TEXT_CON_WIDTH - 2) as usize;
        let mut lines: Vec<String> = Vec::new();
        for line in text_lines {
            if line.chars().count() <= width {
                lines.push(line.clone());
            } else {
                lines.extend(wrap(line, width, " "));
            }
        }
        let shown = (TEXT_CON_HEIGHT - 6) as usize;
        let last = lines.len().saturating_sub(1);
        let top = shown.saturating_sub(1);
        let mut y2 = last;
        loop {
            let t = &mut self.text_con;
            t.clear();
            t.frame(0, 0, TEXT_CON_WIDTH, TEXT_CON_HEIGHT, false, false);
            t.fg = MENU_TITLE_COLOR;
            t.print_ex(TEXT_CON_XM, 1, Align::Center, title);
            t.fg = color::WHITE;
            t.print_ex(TEXT_CON_XM, TEXT_CON_HEIGHT - 3, Align::Center, &format!("[{}] Scroll", hl("Up/Down/PgUp/PgDn/Home/End")));
            t.print_ex(TEXT_CON_XM, TEXT_CON_HEIGHT - 2, Align::Center, &format!("[{}] Return", hl("ESC")));
            let y1 = if lines.len() <= shown { 0 } else { y2 + 1 - shown };
            for (n, line) in lines.iter().enumerate().skip(y1).take(y2 + 1 - y1) {
                t.print(2, 3 + (n - y1) as i32, line);
            }
            Console::blit(&self.text_con, 0, 0, TEXT_CON_WIDTH, TEXT_CON_HEIGHT, &mut self.root, TEXT_CON_X, TEXT_CON_Y);
            self.flush();
            if self.scr.is_headless() {
                break;
            }
            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                let can_scroll = lines.len() > shown;
                match self.key_vk() {
                    Vk::Escape => {
                        Console::blit(&self.con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, &mut self.root, 0, 0);
                        self.flush();
                        return;
                    }
                    Vk::Up if can_scroll && y2 > top => y2 -= 1,
                    Vk::Down if y2 < last => y2 += 1,
                    Vk::Home if can_scroll => y2 = top,
                    Vk::End => y2 = last,
                    Vk::PageUp if can_scroll => y2 = y2.saturating_sub(10).max(top),
                    Vk::PageDown => y2 = (y2 + 10).min(last),
                    _ => {
                        self.flush();
                        continue;
                    }
                }
                refresh = true;
            }
        }
        Console::blit(&self.con, 0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, &mut self.root, 0, 0);
        self.flush();
    }

    /// `ShowCrewInfo`: the crew side by side; Enter for a crewman's
    /// skills, N and K to rename them or set a nickname.
    pub fn show_crew_info(&mut self) {
        self.darken_screen();
        let mut selected = 0usize;
        loop {
            let m = &mut self.menu_con;
            m.bg = color::BLACK;
            m.fg = color::WHITE;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Tank Crew Info");
            m.fg = color::WHITE;
            let hints = [
                format!("[{}] Change Crew Selection", hl("A/D/Left/Right")),
                format!("[{}] View/Add/Upgrade Skills", hl("Enter")),
                format!("Change [{}]ame", hl("N")),
                format!("[{}] Set/Reset Nickname", hl("K")),
                format!("[{}] Return", hl("ESC")),
            ];
            for (n, h) in hints.iter().enumerate() {
                m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 7 + n as i32, Align::Center, h);
            }
            for n in 0..self.tank.crew.len() {
                let x = 2 + n as i32 * CREW_PANEL_W;
                let rank = self.crew_rank_fit(n);
                display_crew_info(&mut self.menu_con, &self.tank.crew[n], &rank, x, 3, n == selected);
            }
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                let vk = self.key_vk();
                let kc = self.key_char().to_ascii_lowercase();
                if vk == Vk::Escape {
                    return self.after_window();
                } else if vk == Vk::Enter {
                    self.show_skills(selected);
                    refresh = true;
                } else if kc == 'd' || vk == Vk::Right {
                    selected = self.tank.next_crew(selected);
                    refresh = true;
                } else if kc == 'a' || vk == Vk::Left {
                    selected = self.tank.prev_crew(selected);
                    refresh = true;
                } else if kc == 'k' {
                    self.con.clear();
                    let text = format!("Enter new nickname for {}", self.crew(selected).name);
                    let nick = self.get_input(&text, SCREEN_YM - 2, NICKNAME_MAX_LEN, &[], false);
                    self.crewm(selected).nickname = nick;
                    refresh = true;
                } else if kc == 'n' {
                    self.con.clear();
                    let text = format!("Enter new name for {}", self.crew(selected).name);
                    let name = self.get_input(&text, SCREEN_YM - 2, NAME_MAX_LEN, &[], true);
                    self.crewm(selected).name = name;
                    refresh = true;
                }
                self.flush();
            }
        }
        self.after_window();
    }

    /// A crewman's full rank, or the short one if the full one is too wide
    /// for a crew panel ("Private First Class").
    fn crew_rank_fit(&self, i: usize) -> String {
        let full = self.crew_rank(i, false);
        if full.chars().count() as i32 <= CREW_PANEL_W - 2 { full } else { self.crew_rank(i, true) }
    }

    /// `ShowSkills`: the skills a crewman's position can take, and spending
    /// skill points on them.
    pub fn show_skills(&mut self, i: usize) {
        if !self.crew(i).alive {
            return;
        }
        let mut selected_skill = 0usize;
        loop {
            let usa = self.campaign.player_nation == "USA";
            let cm = &self.tank.crew[i];
            let skill_list: Vec<(&defs::CrewSkill, i32)> = defs::defs()
                .skills
                .iter()
                .filter(|s| s.restrictions.is_empty() || s.restrictions.contains(&cm.position))
                .filter(|s| s.name != "Gyrostabilizer" || usa)
                .map(|s| (s, cm.skills.iter().find(|k| k.name == s.name).map_or(0, |k| k.level)))
                .collect();
            let gyro_ok = self.campaign.gyro_skill_avail;
            let rank = self.crew_rank_fit(i);

            let m = &mut self.menu_con;
            m.bg = color::BLACK;
            m.fg = color::WHITE;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Crew Skills");
            m.fg = color::WHITE;
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 5, Align::Center, &format!("[{}] Change Skill Selection", hl("W/S/Up/Down")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 4, Align::Center, &format!("[{}] Add/Upgrade Skill", hl("Enter")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 3, Align::Center, &format!("[{}] Return/Continue", hl("ESC")));
            display_crew_info(m, cm, &rank, 1, 3, false);

            let (lx, dx) = (21, 46);
            m.fg = color::LIGHT_GREY;
            m.frame(dx, 3, 52, 20, false, false);
            m.print(dx + 3, 4, "Description");
            m.print(dx + 3, 16, "Activation Levels");
            m.frame(lx, 3, 24, 20, false, false);
            m.print(lx + 2, 4, "Skills");
            m.fg = color::WHITE;

            for (n, (skill, level)) in skill_list.iter().enumerate() {
                let y = 6 + n as i32;
                if n == selected_skill {
                    m.bg = color::DARKER_GREY;
                    m.rect(lx + 1, y, 22, 1, false);
                    m.bg = color::BLACK;
                    m.fg = color::WHITE;
                    for (k, line) in wrap(&skill.desc, 46, " ").iter().enumerate() {
                        m.print(dx + 3, 6 + k as i32, line);
                    }
                    if skill.levels[0] == 100 {
                        m.print(dx + 3, 18, "Always active");
                    } else {
                        let mut x2 = dx + 3;
                        for sl in &skill.levels {
                            m.fg = if sl <= level { color::LIGHT_BLUE } else { color::WHITE };
                            m.print(x2, 18, &format!("{sl}%"));
                            x2 += sl.to_string().len() as i32 + 2;
                        }
                    }
                }
                m.fg = if skill.name == "Gyrostabilizer" && !gyro_ok {
                    GREYED_COLOR
                } else if *level > 0 {
                    color::LIGHT_BLUE
                } else {
                    color::WHITE
                };
                m.print(lx + 2, y, &skill.name);
            }
            m.fg = color::WHITE;
            let choice = skill_list.get(selected_skill).map(|(s, l)| ((*s).clone(), *l));
            let count = skill_list.len();
            self.blit_menu();
            self.flush();
            if self.scr.is_headless() {
                return;
            }

            let mut refresh = false;
            while !refresh {
                self.check_for_event();
                let vk = self.key_vk();
                let kc = self.key_char().to_ascii_lowercase();
                if vk == Vk::Escape {
                    return;
                }
                if vk == Vk::Up || kc == 'w' {
                    selected_skill = if selected_skill == 0 { count - 1 } else { selected_skill - 1 };
                    refresh = true;
                } else if vk == Vk::Down || kc == 's' {
                    selected_skill = if selected_skill + 1 >= count { 0 } else { selected_skill + 1 };
                    refresh = true;
                } else if vk == Vk::Enter {
                    if let Some((skill, level)) = &choice {
                        if !self.crew(i).no_actions() && self.crew(i).skill_pts > 0 && (skill.name != "Gyrostabilizer" || gyro_ok) {
                            self.add_or_upgrade_skill(i, skill, *level);
                        }
                    }
                    refresh = true;
                }
                self.flush();
            }
        }
    }

    /// Spend a skill point on a new skill, or on its next activation level.
    fn add_or_upgrade_skill(&mut self, i: usize, skill: &defs::CrewSkill, level: i32) {
        let name = self.crew(i).name.clone();
        if level == 0 {
            if self.popup_confirm(&format!("Spend a skill point to add skill \"{}\"", skill.name)) {
                let c = self.crewm(i);
                c.skill_pts -= 1;
                c.skills.push(crate::model::SkillRecord { name: skill.name.clone(), level: skill.levels[0] });
                self.write_journal(&format!("{name} added a new skill: {}, now at {}% activation", skill.name, skill.levels[0]));
                self.play_sound("new_skill");
            }
        } else if let Some(&next) = skill.levels.iter().find(|&&l| level < l) {
            if self.popup_confirm(&format!("Spend a skill point to upgrade skill \"{}\"", skill.name)) {
                let c = self.crewm(i);
                c.skill_pts -= 1;
                c.upgrade_skill(&skill.name, next);
                self.write_journal(&format!("{name} upgraded a skill: {}, now at {next}% activation", skill.name));
                self.play_sound("new_skill");
            }
        }
    }
}

/// Crew panel size on the crew screens: five across 100 columns (the
/// original's were 27 wide and 30 tall).
const CREW_PANEL_W: i32 = 19;
const CREW_PANEL_H: i32 = 17;

/// `DisplayCrewInfo`: a crewman's panel. A hometown or wound too long for
/// one line takes two, so the rows below sit lower than the original's.
fn display_crew_info(con: &mut Console, c: &crate::model::Crewman, rank: &str, x: i32, y: i32, highlight: bool) {
    let inner = (CREW_PANEL_W - 2) as usize;
    let fit = |s: &str| s.chars().take(inner).collect::<String>();
    con.fg = if highlight { SELECTED_COLOR } else { color::LIGHT_GREY };
    con.frame(x, y, CREW_PANEL_W, CREW_PANEL_H, false, false);
    con.fg = color::WHITE;
    con.print_ex(x + 1, y + 1, Align::Left, &c.position);
    con.fg = color::LIGHT_GREY;
    con.print_ex(x + 1, y + 2, Align::Left, &fit(rank));
    con.fg = color::WHITE;
    con.print_ex(x + 1, y + 3, Align::Left, &fit(&c.name));
    if !c.nickname.is_empty() {
        con.fg = color::LIGHT_BLUE;
        con.print_ex(x + 1, y + 4, Align::Left, &fit(&format!(" \"{}\"", c.nickname)));
    }
    con.fg = color::LIGHT_GREY;
    for (n, line) in wrap(&c.hometown, inner, "").iter().take(2).enumerate() {
        con.print_ex(x + 1, y + 5 + n as i32, Align::Left, line);
    }
    if c.no_actions() {
        con.fg = color::LIGHT_RED;
        let text = if !c.alive {
            "Dead"
        } else if c.unconscious {
            "Unconscious"
        } else {
            "Stunned"
        };
        con.print_ex(x + 2, y + 7, Align::Left, text);
    }
    let wound = if c.v_serious_wound {
        "Very Serious Wound"
    } else if c.serious_wound {
        "Serious Wound"
    } else if c.light_wound {
        "Light Wound"
    } else {
        ""
    };
    con.fg = color::RED;
    for (n, line) in wrap(wound, inner - 1, "").iter().take(2).enumerate() {
        con.print_ex(x + 2, y + 8 + n as i32, Align::Left, line);
    }

    for (n, dec) in c.decorations.iter().take(inner).enumerate() {
        let x1 = x + 1 + n as i32;
        let (top, bottom) = match dec.as_str() {
            "Purple Heart" => ((31, color::PURPLE, color::BLACK), (3, color::PURPLE, color::BLACK)),
            "Bronze Star" => ((31, color::RED, color::BLACK), (15, color::LIGHT_YELLOW, color::BLACK)),
            "Silver Star" => ((31, color::BLUE, color::BLACK), (15, color::LIGHT_YELLOW, color::BLACK)),
            "Distinguished Service Cross" => ((31, color::BLUE, color::BLACK), (197, color::DARK_YELLOW, color::BLACK)),
            "Congressional Medal of Honor" => ((254, color::LIGHT_BLUE, color::BLACK), (42, color::YELLOW, color::BLACK)),
            "Military Medal" => ((186, color::WHITE, color::BLUE), (42, color::LIGHT_GREY, color::BLACK)),
            "Military Cross" => ((179, color::PURPLE, color::WHITE), (31, color::LIGHT_GREY, color::BLACK)),
            "Distinguished Service Order" => ((179, color::RED, color::BLUE), (31, color::WHITE, color::BLACK)),
            "Victoria Cross" => ((178, color::RED, color::BLACK), (31, color::DARK_YELLOW, color::BLACK)),
            _ => continue,
        };
        con.put_char_ex(x1, y + 10, top.0, top.1, top.2);
        con.put_char_ex(x1, y + 11, bottom.0, bottom.1, bottom.2);
    }

    con.fg = color::LIGHT_GREY;
    con.print_ex(x + 1, y + 13, Align::Left, "Level:");
    con.print_ex(x + 1, y + 14, Align::Left, "Exp:");
    con.print_ex(x + 1, y + 15, Align::Left, "Skill Pts:");
    con.fg = color::WHITE;
    con.print_ex(x + 8, y + 13, Align::Left, &c.level.to_string());
    con.print_ex(x + 6, y + 14, Align::Left, &format!("{}/{}", c.exp, Game::get_exp_req(c.level + 1)));
    if c.skill_pts > 0 {
        con.fg = color::LIGHT_BLUE;
    }
    con.print_ex(x + 12, y + 15, Align::Left, &c.skill_pts.to_string());
    con.fg = color::WHITE;
}
