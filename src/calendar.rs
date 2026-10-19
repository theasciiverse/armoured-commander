//! The campaign around the days of action, from armcom.py: starting one
//! (`NewCampaign`, `ChooseCampaign`, `SetCampaignSettings`, the tank and
//! commander names), the calendar (`RunCalendar`, `AdvanceDay`), what
//! happens between days (`CheckPlayerTank`, `CheckAwardsPromotions`), and
//! its end (`CheckCommander`, `AddHighScore`, `RecordJournal`).
//!
//! The calendar screen: the day's text in a 40-column panel on
//! the left, and the campaign's overview map on the right: as drawn (85x50)
//! when the screen has room, otherwise in half blocks (85x25), in a window
//! that follows the day's location marker if it is narrower than the map.

use crate::bones;
use crate::color::{self, Color};
use crate::console::{Align, Console};
use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::model::*;
use crate::ui::{Vk, wrap};

/// Where the overview map's window sits on the calendar screen.
const CAL_MAP_X: i32 = LEFT_W + 1;
const CAL_MAP_Y: i32 = 2;

/// The space for the overview map, right of the text panel.
fn cal_map_w() -> i32 {
    sw() - CAL_MAP_X
}

/// The overview map drawn in half blocks at `cols` x `rows` cells: two
/// pixels a cell, `▀` with the upper colour over the lower, each sampled
/// from the nearest cell of the map. At the map's own width and half its
/// height, each pair of rows becomes one. Text (place names, the title)
/// is kept at its size, each run of it (the gaps between words included)
/// where its first letter falls.
fn scaled_blocks(src: &Console, cols: i32, rows: i32) -> Console {
    let mut out = Console::new(cols, rows);
    let colour = |x: i32, y: i32| match src.cell(x, y) {
        Some(c) if (219..=223).contains(&c.ch) => c.fg,
        Some(c) => c.bg,
        None => color::BLACK,
    };
    let (px_h, src_y) = (rows * 2, |py: i32| py * src.h / (rows * 2));
    for y in 0..rows {
        for x in 0..cols {
            let sx = x * src.w / cols;
            out.put_char_ex(x, y, 223, colour(sx, src_y(2 * y)), colour(sx, src_y((2 * y + 1).min(px_h - 1))));
        }
    }
    let is_text = |ch: u8| !matches!(ch, 0 | 32 | 219..=223);
    for sy in 0..src.h {
        let mut sx = 0;
        while sx < src.w {
            if !src.cell(sx, sy).is_some_and(|c| is_text(c.ch)) {
                sx += 1;
                continue;
            }
            let (x0, y0) = (sx * cols / src.w, sy * px_h / src.h / 2);
            // a run goes on over a one-cell gap between words, which the
            // art fills in with a block ("Patton's Best")
            let text_at = |x: i32| src.cell(x, sy).is_some_and(|c| is_text(c.ch));
            let mut n = 0;
            while text_at(sx) || (sx + 1 < src.w && text_at(sx + 1)) {
                let c = src.cell(sx, sy).expect("in the map");
                if is_text(c.ch) {
                    out.put_char_ex(x0 + n, y0, c.ch, c.fg, c.bg);
                } else {
                    out.put_char_ex(x0 + n, y0, b' ', c.fg, colour(sx, sy));
                }
                sx += 1;
                n += 1;
            }
        }
    }
    out
}

/// How big to draw the overview map on the calendar: as big as the space
/// allows, in the proportions of the original's 8x12-pixel cells (a half
/// block is about square); never smaller than half blocks at the map's own
/// width, which shows in a window if the screen is narrower.
fn cal_map_size(src: &Console, screen_w: i32, screen_h: i32) -> (i32, i32) {
    let room_rows = screen_h - 4 - CAL_MAP_Y;
    let room_w = screen_w - CAL_MAP_X - 2;
    let aspect = (src.w * 8) as f64 / (src.h * 12) as f64;
    let mut px_h = room_rows * 2;
    let mut w = (px_h as f64 * aspect).round() as i32;
    if w > room_w {
        w = room_w;
        px_h = (w as f64 / aspect).round() as i32;
    }
    if w <= src.w { (src.w, (src.h + 1) / 2) } else { (w, (px_h + 1) / 2) }
}

/// The clock's date and time (UTC; the original used local time).
fn now() -> (i64, u32, u32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (y, m, d) = crate::title::civil_from_days(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    (y, m, d, (s / 3600) as u32, (s / 60 % 60) as u32)
}

const MONTHS: [&str; 13] = [
    "", "January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November",
    "December",
];

impl Game {
    /// `NewCampaign`: choose a campaign and its settings, the first day,
    /// the tank and crew, then on to the calendar. The display settings
    /// carry over from the last campaign (the original reset them).
    pub fn new_campaign(&mut self) {
        self.tutorial_message("welcome");
        let old = std::mem::replace(&mut self.campaign, Campaign::new());
        self.campaign.animations = old.animations;
        self.campaign.sounds = old.sounds;
        self.campaign.pause_labels = old.pause_labels;
        self.campaign.tutorial_message = old.tutorial_message;
        self.battle = None;

        let file = self.choose_campaign();
        let Some(def) = defs::CAMPAIGN_FILES.iter().find(|(f, _)| *f == file).and_then(|(_, xml)| defs::load_campaign(xml).ok())
        else {
            self.popup("Error: No Campaign files found!");
            return;
        };
        self.campaign.campaign_file = file;
        log::info!("campaign: {}", self.campaign.campaign_file);

        self.write_journal("*** Armoured Commander Campaign Journal ***");
        self.write_journal(&format!("Program Version: {VERSION}{SUBVERSION}"));
        self.write_journal("");
        let (y, m, d, hh, mm) = now();
        let (h12, ampm) = (if hh % 12 == 0 { 12 } else { hh % 12 }, if hh < 12 { "AM" } else { "PM" });
        self.write_journal(&format!("Campaign Started at {h12:02}:{mm:02}{ampm} on {} {d:02}, {y}", MONTHS[m as usize]));
        self.write_journal("");

        self.campaign.load_info(&def);
        let name = self.campaign.campaign_name.clone();
        self.write_journal(&format!("Campaign Name: {name}"));

        self.set_campaign_settings();
        let on = |b: bool| if b { "On" } else { "Off" };
        let text = format!(" Unlimited Tank Selection: {}", on(self.campaign.unlimited_tank_selection));
        self.write_journal(&text);
        let text = format!(" Casual Commander Replacement: {}", on(self.campaign.casual_commander));
        self.write_journal(&text);
        self.write_journal("");

        self.advance_day();

        let tank_type = if self.campaign.unlimited_tank_selection { self.show_tank_info(true) } else { None };
        self.tank = PlayerTank::new(tank_type.as_deref());
        self.set_tank_stats();
        self.get_tank_name();
        let text = format!("Starting Tank: {} \"{}\"", self.tank.unit_type, self.tank.name);
        self.write_journal(&text);

        if let Some(c) = self.spawn_crew_member(None, "Commander", 3) {
            self.crew_set_level(c, 3);
            self.get_commander_name(c);
            let text = format!(
                "{} is assigned as your tank Commander. You may add/upgrade his skills now or save your skill points \
                 for later.",
                self.crew(c).name
            );
            self.popup(&text);
            self.show_skills(c);
        }
        if let Some(c) = self.spawn_crew_member(None, "Gunner", 2) {
            self.crew_set_level(c, 2);
            let text = format!("{} is assigned as your Gunner.", self.crew(c).name);
            self.popup(&text);
            self.show_skills(c);
        }
        for (position, rank, title) in [("Loader", 1, "Loader"), ("Driver", 0, "Driver"), ("Asst. Driver", 0, "Assistant Driver")] {
            if let Some(c) = self.spawn_crew_member(None, position, rank) {
                let text = format!("{} is assigned as your {title}.", self.crew(c).name);
                self.popup(&text);
                self.show_skills(c);
            }
        }

        self.run_calendar(false);
    }

    /// `ChooseCampaign`: the campaign files, reverse alphabetical by file
    /// name; returns the chosen file.
    pub fn choose_campaign(&mut self) -> String {
        let mut list: Vec<(String, String, String)> = defs::CAMPAIGN_FILES
            .iter()
            .filter_map(|(f, xml)| defs::load_campaign(xml).ok().map(|d| (f.to_string(), d.name, d.description)))
            .collect();
        list.sort_by(|a, b| b.cmp(a));
        let mut selected = 0usize;
        self.root.clear();
        loop {
            let (_, name, description) = &list[selected];
            let m = &mut self.menu_con;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Select a Campaign");
            m.fg = SELECTED_COLOR;
            m.print_ex(MENU_CON_XM, 8, Align::Center, name);
            m.fg = color::WHITE;
            for (n, line) in wrap(description, 80, " ").iter().enumerate() {
                m.print_ex(MENU_CON_XM - 40, 10 + n as i32, Align::Left, line);
            }
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 3, Align::Center, &format!("[{}] Select Different Campaign", hl("A/D/Left/Right")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Continue with Selected Campaign", hl("Enter")));
            self.blit_menu();
            if self.scr.is_headless() {
                // tests play either campaign
                let n = self.rng.int(0, list.len() as i32 - 1) as usize;
                return list[n].0.clone();
            }

            let mut refresh = false;
            while !refresh {
                self.flush();
                self.check_for_event();
                let kc = self.key_char().to_ascii_lowercase();
                let vk = self.key_vk();
                if vk == Vk::Left || kc == 'a' {
                    selected = if selected > 0 { selected - 1 } else { list.len() - 1 };
                    refresh = true;
                } else if vk == Vk::Right || kc == 'd' {
                    selected = if selected + 1 < list.len() { selected + 1 } else { 0 };
                    refresh = true;
                } else if vk == Vk::Enter {
                    return list[selected].0.clone();
                }
            }
        }
    }

    /// `SetCampaignSettings`: tank selection, commander replacement and the
    /// start date, fixed for the campaign.
    pub fn set_campaign_settings(&mut self) {
        self.root.clear();
        loop {
            let c = &self.campaign;
            let m = &mut self.menu_con;
            m.clear();
            m.align = Align::Left;
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.fg = MENU_TITLE_COLOR;
            m.print_ex(MENU_CON_XM, 1, Align::Center, "Campaign Settings");
            m.fg = color::WHITE;
            m.print_ex(MENU_CON_XM, 3, Align::Center, "These settings cannot be changed once a campaign has begun");
            m.print_ex(MENU_CON_XM, 4, Align::Center, "Your selections will be displayed alongside your final score");

            let x = 8;
            let tanks = if c.unlimited_tank_selection { "Unlimited" } else { "Strict" };
            m.fg = MENU_TITLE_COLOR;
            m.print(x, 7, &format!("[{}]ank Selection: {tanks}", hl("T")));
            m.fg = color::WHITE;
            let lines: &[&str] = if c.unlimited_tank_selection {
                &[
                    "You may choose any available model of tank when starting your campaign, when",
                    " replacing a destroyed tank, or during a refit period.",
                ]
            } else {
                &[
                    "You must begin the campaign with the assigned tank model, and must accept the",
                    " model offered to you when replacing a destroyed tank. One randomly selected model",
                    " will be offered to you during each refit period. Tank model selection is weighted",
                    " by historical availability and rarity.",
                ]
            };
            for (n, line) in lines.iter().enumerate() {
                m.print(x, 9 + n as i32, line);
            }

            let cmdr = if c.casual_commander { "Casual" } else { "Realistic" };
            m.fg = MENU_TITLE_COLOR;
            m.print(x, 15, &format!("[{}]ommander Replacement: {cmdr}", hl("C")));
            m.fg = color::WHITE;
            if c.casual_commander {
                m.print(x, 17, "If your tank commander is killed or sent home due to injuries, he will be replaced");
                m.print(x, 18, " by a new crewman and you may continue playing.");
            } else {
                m.print(x, 17, "If your tank commander is killed or sent home due to injuries, your campaign is over.");
            }

            let day = &c.days[c.start_date];
            m.fg = MENU_TITLE_COLOR;
            m.print(x, 21, &format!("Campaign Start Date: {}", c.get_date(Some([day.year, day.month, day.date]))));
            m.fg = color::WHITE;
            m.print(x, 23, &format!("[{}] to change starting date of campaign.", hl("A/D")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 3, Align::Center, &format!("[{}] Toggle Setting", hl("C/T")));
            m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] Continue", hl("Enter")));
            self.blit_menu();
            if self.scr.is_headless() {
                // tests play with every setting
                let c = &mut self.campaign;
                c.unlimited_tank_selection = self.rng.int(0, 1) == 1;
                c.casual_commander = self.rng.int(0, 1) == 1;
                c.start_date = self.rng.int(0, c.days.len() as i32 - 1) as usize;
                return;
            }

            let mut refresh = false;
            while !refresh {
                self.flush();
                self.check_for_event();
                if self.key_vk() == Vk::Enter {
                    return;
                }
                let c = &mut self.campaign;
                match self.scr.key.c.to_ascii_lowercase() {
                    't' => c.unlimited_tank_selection = !c.unlimited_tank_selection,
                    'c' => c.casual_commander = !c.casual_commander,
                    'a' => c.start_date = if c.start_date == 0 { c.days.len() - 1 } else { c.start_date - 1 },
                    'd' => c.start_date = if c.start_date + 1 < c.days.len() { c.start_date + 1 } else { 0 },
                    _ => continue,
                }
                refresh = true;
            }
        }
    }

    /// `GetTankName`
    pub fn get_tank_name(&mut self) {
        self.con.bg = color::BLACK;
        self.con.clear();
        let names = defs::defs().tank_names.clone();
        let mut name = self.get_input("Choose a name for your Sherman tank", sym() - 2, 17, &names, false);
        if name.is_empty() {
            name = self.rng.choice(&names).clone();
        }
        self.tank.name = name;
    }

    /// `GetCommanderName`
    pub fn get_commander_name(&mut self, i: usize) {
        self.con.bg = color::BLACK;
        self.con.clear();
        let name = self.get_input("Enter your name", sym() - 2, NAME_MAX_LEN, &[], true);
        self.crewm(i).name = if name.is_empty() { self.generate_name() } else { name };
    }

    /// `RandomPlayerTankModel`: a Sherman weighted by its rarity this month.
    pub fn random_player_tank_model(&mut self) -> String {
        let mut list: Vec<(String, i32)> = self
            .campaign
            .player_veh_list
            .iter()
            .map(|v| (v.clone(), self.campaign.get_rf(v)))
            .filter(|(_, rf)| *rf > 0)
            .collect();
        self.shuffle(&mut list);
        let total: i32 = list.iter().map(|(_, rf)| rf).sum();
        let mut result = self.rng.int(1, total.max(1));
        for (v, rf) in list {
            if result <= rf {
                return v;
            }
            result -= rf;
        }
        log::error!("could not randomly choose a new tank model");
        "M4 Turret A".into()
    }

    /// `CheckPlayerTankPositions`: a new assistant driver for a tank that
    /// has the position, or one reassigned from a tank that has not.
    pub fn check_player_tank_positions(&mut self) {
        let no_asst = self.tank.stats.has("no_asst_driver");
        let asst = self.crew_pos("Asst. Driver");
        if !no_asst && asst.is_none() {
            let highest = self.get_highest_crew_level();
            if let Some(n) = self.spawn_crew_member(None, "Asst. Driver", 0) {
                let level = self.rng.int(1, highest);
                self.crew_set_level(n, level);
                let text = format!("{} joins the tank crew as the Assistant Driver.", self.crew(n).name);
                self.popup(&text);
                self.write_journal(&text);
                self.show_skills(n);
            }
        } else if let (true, Some(a)) = (no_asst, asst) {
            let c = self.tank.crew.remove(a);
            let text = format!("Your new tank has no assistant driver position. {} is reassigned elsewhere.", c.name);
            self.popup(&text);
            self.write_journal(&text);
        }
        if self.campaign.selected_crew.is_some_and(|s| s >= self.tank.crew.len()) {
            self.campaign.selected_crew = Some(0);
        }
    }

    /// A new tank, after a refit or a loss: fresh, named, and the crew back
    /// on their default orders.
    fn new_tank(&mut self, tank_type: String) {
        self.tank.unit_type = tank_type;
        self.tank.setup();
        self.set_tank_stats();
        self.get_tank_name();
    }

    /// The crew's orders, hatches and spotting for a new tank.
    fn reset_crew_for_new_tank(&mut self) {
        for i in 0..self.tank.crew.len() {
            let c = self.crewm(i);
            c.order = c.default_order.clone();
            self.crew_check_hatch(i);
            self.crew_set_spot_ability(i);
        }
    }

    /// `CheckPlayerTank`: after a day of action, awards; a new tank if the
    /// old one was lost, repairs if not; light and serious wounds heal.
    pub fn check_player_tank(&mut self) {
        self.check_awards_promotions(false);
        if !self.tank.alive || self.tank.swiss_cheese {
            self.replace_crew();
            let tank_type = if self.campaign.unlimited_tank_selection {
                self.show_tank_info(true).unwrap_or_else(|| self.tank.unit_type.clone())
            } else {
                let t = self.random_player_tank_model();
                self.popup(&format!("HQ assigns your crew to a {t} tank."));
                t
            };
            self.tank.alive = true;
            self.new_tank(tank_type);
            let text = format!("New player tank: {} \"{}\"", self.tank.unit_type, self.tank.name);
            self.write_journal(&text);
            self.check_player_tank_positions();
            self.reset_crew_for_new_tank();
        } else {
            if self.tank.immobilized {
                self.tank.immobilized = false;
                self.popup("Your tank is repaired for the next day of combat.");
            }
            self.tank.damage_list.clear();
        }
        for c in &mut self.tank.crew {
            c.light_wound = false;
            c.serious_wound = false;
        }
    }

    /// `CheckAwardsPromotions`: Purple Hearts; at the start of a month, a
    /// decoration for the commander by the month's best day; promotion by
    /// campaign VP.
    pub fn check_awards_promotions(&mut self, new_month: bool) {
        if self.campaign.player_nation == "USA" {
            for i in 0..self.tank.crew.len() {
                let c = self.crew(i);
                if c.serious_wound || c.v_serious_wound || !c.alive {
                    self.crew_award_decoration(i, "Purple Heart");
                }
            }
        }
        let Some(cmdr) = self.crew_pos("Commander") else { return };
        if new_month {
            let (_, _, roll) = self.roll_2d6();
            let award_score = roll + self.campaign.record_day_vp;
            let award = self.campaign.decorations.iter().rev().find(|d| award_score >= d.2).map(|d| d.0.clone());
            if let Some(name) = award {
                self.crew_award_decoration(cmdr, &name);
            }
            self.campaign.record_day_vp = 0;
        }
        if !self.crew(cmdr).alive {
            return;
        }
        for n in (0..self.campaign.ranks.len()).rev() {
            if n == self.crew(cmdr).rank_level {
                break;
            }
            if self.campaign.vp >= self.campaign.ranks[n].2 {
                self.crewm(cmdr).rank_level = n;
                let rank = self.crew_rank(cmdr, false);
                let text = format!("{} promoted to rank of {rank}", self.crew(cmdr).name);
                self.write_journal(&text);
                self.popup(&format!(
                    "Congratulations, commander. Due to your continued service and leadership, you have been promoted \
                     to the rank of {rank}"
                ));
                break;
            }
        }
    }

    /// `CheckCommander`: true if the campaign ends because the commander
    /// is dead or sent home (unless the casual commander option is on):
    /// the score, the journal and the final stats. Once over, it is not
    /// ended again.
    pub fn check_commander(&mut self) -> bool {
        if self.campaign.casual_commander {
            return false;
        }
        if self.campaign.over {
            return true;
        }
        let Some(c) = self.crew_pos("Commander") else { return false };
        let (alive, vsw) = (self.crew(c).alive, self.crew(c).v_serious_wound);
        if alive && !vsw {
            return false;
        }
        self.campaign.over = true;
        crate::save::delete();
        if !alive {
            self.popup("You are dead. Your campaign is over");
        } else {
            self.popup("You have been seriously injured and are sent home. Your campaign is over.");
        }
        self.add_high_score();
        let why = if vsw { "Commander was very seriously wounded and was sent home." } else { "Commander was killed in action." };
        self.write_journal(&format!("Campaign Over: {why}"));
        self.journal_final_stats();
        self.record_journal();
        self.show_campaign_stats();
        true
    }

    /// The final stats, as `CheckCommander` writes them to the journal.
    fn journal_final_stats(&mut self) {
        self.write_journal("");
        self.write_journal("******");
        self.write_journal("Final Campaign Stats");
        let c = &self.campaign;
        let stat = |name: &str| c.stats.iter().find(|(k, _)| k == name).map_or(0, |(_, v)| *v);
        let mut lines = vec![
            format!("Days of Combat: {}/{}", stat("Days of Combat"), c.days.len()),
            format!("Total Victory Points: {}", c.vp + c.day_vp),
        ];
        for name in &defs::defs().c_stats {
            lines.push(format!("{name}: {}", stat(name)));
        }
        for line in lines {
            self.write_journal(&line);
        }
        self.write_journal("");
        self.write_journal("******");
    }

    /// `AddHighScore`
    pub fn add_high_score(&mut self) {
        let Some(c) = self.crew_pos("Commander") else { return };
        let cm = self.crew(c);
        let outcome = if !cm.alive {
            format!("KIA on {}", self.campaign.get_date(None))
        } else if cm.v_serious_wound {
            format!("Sent Home on {}", self.campaign.get_date(None))
        } else {
            "Survived".into()
        };
        let c = &self.campaign;
        bones::add_high_score(bones::Score {
            tank_name: self.tank.name.clone(),
            commander_name: cm.name.clone(),
            vp: c.vp + c.day_vp,
            outcome,
            unlimited_tank_selection: c.unlimited_tank_selection,
            casual_commander: c.casual_commander,
            campaign_name: c.campaign_name.clone(),
        });
    }

    /// `RecordJournal`: the crew's reports, then the whole journal to a
    /// text file in the player's data folder.
    pub fn record_journal(&mut self) {
        for i in 0..self.tank.crew.len() {
            for line in self.crew_generate_report(i) {
                self.write_journal(&line);
            }
        }
        let Some(dir) = bones::data_dir() else { return };
        let (y, m, d, hh, mm) = now();
        let path = dir.join(format!("Armoured_Commander_Journal_{hh:02}_{mm:02}_{d:02}-{m:02}-{y}.txt"));
        let mut text = self.campaign.campaign_journal.join("\n");
        text.push('\n');
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::write(&path, text) {
            Ok(()) => log::info!("journal written to {}", path.display()),
            Err(e) => log::error!("could not write the journal: {e}"),
        }
    }

    /// Fade text in from black on the root console, as `AdvanceDay` does.
    fn fade_in_text(&mut self, y: i32, text: &str) {
        if self.campaign.animations && !self.scr.is_headless() {
            for c in (0..255).step_by(5) {
                self.root.fg = Color(c as u8, c as u8, c as u8);
                self.root.print_ex(sxm(), y, Align::Center, text);
                self.flush();
                self.wait(2);
            }
        }
        self.root.fg = color::WHITE;
        self.root.print_ex(sxm(), y, Align::Center, text);
        self.flush();
    }

    /// `console_set_fade` down to black.
    fn fade_out(&mut self) {
        if !self.campaign.animations || self.scr.is_headless() {
            return;
        }
        let screen = self.root.clone();
        for i in (1..=255).rev().step_by(10) {
            self.root = screen.clone();
            self.root.darken(1.0 - i as f32 / 255.0);
            self.flush();
            self.wait(1);
        }
        self.root = screen;
        self.root.clear();
        self.flush();
    }

    /// `AdvanceDay`: on to the next day in the calendar (or the start date),
    /// with its date and comment faded in; set up the day's mission, or
    /// the refit's tank offer. At the end of the calendar, the campaign
    /// is over.
    pub fn advance_day(&mut self) {
        const DATE_Y: i32 = 12;
        self.root.clear();
        self.flush();

        if self.campaign.current_date == [0, 0, 0] {
            let start = self.campaign.start_date;
            self.campaign.set_today(start);
        } else {
            let n = self.campaign.today_index().unwrap_or(0);
            if n + 1 < self.campaign.days.len() {
                if self.campaign.current_date[1] != self.campaign.days[n + 1].month {
                    self.check_awards_promotions(true);
                }
            } else {
                self.popup("You have survived and reached the end of the campaign! Congratulations!");
                self.campaign.over = true;
                self.add_high_score();
                crate::save::delete();
                self.write_journal("Campaign Over: End of campaign calendar");
                self.record_journal();
                self.show_campaign_stats();
                self.campaign.exiting = true;
                return;
            }
            self.campaign.set_today(n + 1);
        }
        self.root.clear();
        self.flush();

        let date_text = self.campaign.get_date(None);
        log::info!("calendar: {date_text}");
        self.fade_in_text(DATE_Y, &date_text);

        let today = self.campaign.today().cloned().expect("today is in the calendar");
        self.write_journal("");
        self.write_journal("******");
        let text = if today.comment == "Refitting" {
            format!("{date_text}: Start of refitting period")
        } else {
            format!("{date_text}: {}", today.comment)
        };
        self.write_journal(&text);
        self.fade_in_text(DATE_Y + 3, &today.comment);

        if today.comment == "Refitting" {
            self.campaign.gyro_skill_avail = true;
            self.campaign.action_day = false;
            if !self.campaign.unlimited_tank_selection {
                self.campaign.tank_on_offer = self.random_player_tank_model();
            }
        } else {
            self.campaign.tank_on_offer = String::new();
            self.campaign.action_day = true;
            self.campaign.scen_res = match today.resistance_level.as_deref() {
                Some("L") => "Light",
                Some("M") => "Medium",
                _ => "Heavy",
            }
            .into();
            self.campaign.scen_type = match today.mission.as_deref() {
                Some("A") => "Advance",
                Some("B") => "Battle",
                _ => "Counterattack",
            }
            .into();
            self.write_journal("Action Day:");
            let text = format!(" Mission: {}", self.campaign.scen_type);
            self.write_journal(&text);
            let text = format!(" Expected Resistance: {}", self.campaign.scen_res);
            self.write_journal(&text);
            let (h, m) = self.campaign.get_sunrise();
            self.write_journal(&format!(" Sun rose at {h}:{m:02}"));
        }

        self.wait(200);
        self.fade_out();
    }

    /// The calendar screen.
    fn draw_calendar(&mut self, campaign_map: Option<&Console>) {
        let today = self.campaign.today().cloned().unwrap_or_default();
        self.con.bg = color::BLACK;
        self.con.clear();
        self.display_menu_bar();
        let camp = &self.campaign;
        let c = &mut self.con;
        c.fg = color::DARK_GREY;
        c.hline(0, 1, sw());
        c.fg = color::LIGHT_GREY;
        c.frame(1, 2, LEFT_W - 1, sh() - 3, false, false);
        let xm = LEFT_W / 2;
        c.hline(4, 4, LEFT_W - 7);

        c.fg = MENU_TITLE_COLOR;
        c.print_ex(xm, 3, Align::Center, &camp.get_date(None));
        c.fg = color::WHITE;
        c.print_ex(xm, 5, Align::Center, &today.comment);

        let x = 3;
        if !camp.tank_on_offer.is_empty() {
            c.print_ex(x, 7, Align::Left, "HQ offers your crew a transfer to a");
            c.print_ex(x, 8, Align::Left, "new tank:");
            Game::show_vehicle_type_info(c, &camp.tank_on_offer, x + 1, 10, true);
        } else if today.comment == "Refitting" && camp.unlimited_tank_selection {
            c.print_ex(x, 9, Align::Left, "You have the option of switching");
            c.print_ex(x, 10, Align::Left, " to a new tank model during the refit.");
            c.print_ex(x, 11, Align::Left, &format!("Press [{}] to select a new model.", hl("S")));
        } else if let Some(desc) = &today.description {
            for (n, line) in wrap(desc, 35, " ").iter().enumerate() {
                let y = 7 + n as i32;
                if y >= 20 {
                    break;
                }
                c.print_ex(x, y, Align::Left, line);
            }
        }

        if camp.action_day {
            c.print_ex(x, 21, Align::Left, &format!("Mission: {}", camp.scen_type));
            c.print_ex(x, 22, Align::Left, &format!("Resistance: {}", camp.scen_res));
            Game::display_weather(c, &camp.weather, 27, 21);
            c.fg = color::WHITE;
            c.bg = color::BLACK;
        }
        c.print_ex(xm, 26, Align::Center, &format!("Current VP Score: {}", camp.vp));

        if let Some(src) = campaign_map {
            let room = sh() - 4 - CAL_MAP_Y;
            let (cols, rows) = cal_map_size(src, sw(), sh());
            let map = &scaled_blocks(src, cols, rows);
            let mx = today.map_x.as_deref().and_then(|s| s.trim().parse::<i32>().ok()).map(|x| x * cols / src.w);
            let my = today.map_y.as_deref().and_then(|s| s.trim().parse::<i32>().ok()).map(|y| y * rows * 2 / src.h / 2);
            // the whole map, centred, when there is room for it; otherwise a
            // window onto it that follows the marker
            let w = map.w.min(cal_map_w());
            let ox = mx.map_or(0, |x| (x - w / 2).clamp(0, map.w - w));
            let left = CAL_MAP_X + (cal_map_w() - w) / 2;
            let top = CAL_MAP_Y + (room - map.h).max(0) / 2;
            Console::blit(map, ox, 0, w, map.h, c, left, top);
            if let (Some(mx), Some(my)) = (mx, my) {
                c.fg = color::WHITE;
                c.put_char(mx - ox + left, my + top, b'@');
            }
        }

        let y = sh() - 2;
        let first = if camp.action_day {
            format!("[{}]egin combat day", hl("B"))
        } else {
            format!("[{}]dvance to next day", hl("A"))
        };
        c.print_ex(CAL_MAP_X + 1, y, Align::Left, &first);
        c.print_ex(CAL_MAP_X + 30, y, Align::Left, &format!("[{}]iew Tank", hl("V")));
        if today.comment == "Refitting" {
            if !camp.tank_on_offer.is_empty() {
                c.print_ex(CAL_MAP_X + 1, y + 1, Align::Left, &format!("[{}]witch to Offered Tank", hl("S")));
            } else if camp.unlimited_tank_selection {
                c.print_ex(CAL_MAP_X + 1, y + 1, Align::Left, &format!("[{}]witch to New Tank", hl("S")));
            }
        }
        c.print_ex(CAL_MAP_X + 30, y + 1, Align::Left, &format!("Save and [{}]uit", hl("Q")));
        self.blit_con();
        self.flush();
    }

    /// `RunCalendar`: the calendar between days of action, until the
    /// campaign ends or the player quits; with `load`, the saved game,
    /// carrying on any day of action it was saved in.
    pub fn run_calendar(&mut self, load: bool) {
        if load {
            if let Err(e) = self.load_game() {
                log::error!("could not load the saved game: {e}");
                self.popup(&format!("The saved game could not be loaded: {e}"));
                return;
            }
            self.resume_loaded();
        } else {
            self.save_game();
            self.calendar_loop();
        }
    }

    /// After loading: finish any day of action the game was saved in, then
    /// the calendar.
    pub fn resume_loaded(&mut self) {
        if self.campaign.day_in_progress {
            self.init_campaign_day(true);
            if self.campaign.exiting {
                return;
            }
            self.campaign.day_in_progress = false;
            self.check_player_tank();
            self.campaign.action_day = false;
        }
        self.calendar_loop();
    }

    fn calendar_loop(&mut self) {
        let campaign_map = (!self.campaign.map_file.is_empty())
            .then(|| crate::xp::by_name(&self.campaign.map_file))
            .flatten();

        let mut steps = 0;
        loop {
            if self.campaign.exiting {
                return;
            }
            self.draw_calendar(campaign_map.as_ref());
            let refitting = self.campaign.today().is_some_and(|d| d.comment == "Refitting");

            let (vk, kc) = if self.scr.is_headless() {
                steps += 1;
                assert!(steps < 1000, "the calendar does not end");
                (Vk::Char, self.calendar_autopilot(refitting))
            } else {
                self.check_for_event();
                (self.key_vk(), self.key_char().to_ascii_lowercase())
            };

            match vk {
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

            if kc == 'q' {
                self.save_game();
                return;
            } else if kc == 'v' {
                self.campaign.resupply = true;
                self.campaign_view_tank(false);
                self.campaign.resupply = false;
            } else if kc == 'a' {
                if !self.campaign.action_day {
                    self.campaign.saw_action = false;
                    self.advance_day();
                    self.save_game();
                }
            } else if kc == 'b' && self.campaign.action_day {
                self.campaign.tank_on_offer = String::new();
                self.init_campaign_day(false);
                if self.campaign.exiting {
                    return;
                }
                self.campaign.day_in_progress = false;
                self.check_player_tank();
                self.campaign.action_day = false;
                self.campaign.saw_action = true;
            }

            if refitting && kc == 's' && (!self.campaign.tank_on_offer.is_empty() || self.campaign.unlimited_tank_selection) {
                let sure = self.scr.is_headless() || self.popup_confirm("Switching to a new tank cannot be undone. Are you sure?");
                if sure {
                    let tank_type = if self.campaign.unlimited_tank_selection {
                        self.show_tank_info(true).unwrap_or_else(|| self.tank.unit_type.clone())
                    } else {
                        std::mem::take(&mut self.campaign.tank_on_offer)
                    };
                    self.new_tank(tank_type);
                    self.check_player_tank_positions();
                    self.reset_crew_for_new_tank();
                }
            }
            self.flush();
        }
    }

    /// Headless play for tests: begin each day of action, advance past the
    /// rest, and take the offered tank at a refit now and then.
    fn calendar_autopilot(&mut self, refitting: bool) -> char {
        if refitting && !self.campaign.tank_on_offer.is_empty() && self.rng.int(1, 3) == 1 {
            's'
        } else if self.campaign.action_day {
            'b'
        } else {
            'a'
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;
    use crate::ui::Screen;

    /// Whole campaigns, headless, from New Campaign to the end: the
    /// calendar runs out or the commander falls. Nothing may panic.
    #[test]
    fn campaigns_play_to_the_end() {
        let seeds: u64 = std::env::var("ARMCOM_CAMPAIGNS").ok().and_then(|s| s.parse().ok()).unwrap_or(6);
        for seed in 1..=seeds {
            let mut g = Game::new(Screen::headless(), Rng::new(seed * 104_729));
            g.campaign.animations = false;
            g.campaign.pause_labels = false;
            g.new_campaign();
            let c = &g.campaign;
            assert!(c.over, "seed {seed}: campaign left unfinished");
            assert!(g.battle.is_none());
            let stat = |name: &str| c.stats.iter().find(|(k, _)| k == name).map_or(0, |(_, v)| *v);
            eprintln!(
                "seed {seed}: {} from {}, {} days of combat, {} areas captured, {} VP, {}",
                c.campaign_name,
                c.days[c.start_date].comment,
                stat("Days of Combat"),
                stat("Map Areas Captured"),
                c.vp + c.day_vp,
                c.campaign_journal.iter().rev().find(|l| l.starts_with("Campaign Over")).map_or("", |l| l.as_str()),
            );
        }
    }

    /// Saves taken all through headless campaigns (on the calendar, during
    /// days, in the middle of battles) load into a fresh game and play on
    /// to the end.
    #[test]
    fn saved_games_load_and_play_on() {
        let mut checked = (0, 0, 0);
        for seed in 1..=3u64 {
            let mut g = Game::new(Screen::headless(), Rng::new(seed * 7_368_787));
            g.campaign.animations = false;
            g.campaign.pause_labels = false;
            g.test_saves = Some(Vec::new());
            g.new_campaign();
            let saves = g.test_saves.take().unwrap();
            assert!(!saves.is_empty());
            // the first of each kind: in battle, during a day, on the calendar
            let kind = |text: &str| {
                let (c, _, b) = crate::save::from_str(text).unwrap();
                if b.is_some() { 0 } else if c.day_in_progress { 1 } else { 2 }
            };
            let mut picks = Vec::new();
            for k in 0..3 {
                if let Some(n) = saves.iter().position(|t| kind(t) == k) {
                    picks.push(n);
                }
            }
            for n in picks {
                let text = &saves[n];
                let (campaign, tank, battle) = crate::save::from_str(text).unwrap_or_else(|e| panic!("seed {seed} save {n}: {e}"));
                if battle.is_some() {
                    checked.0 += 1;
                } else if campaign.day_in_progress {
                    checked.1 += 1;
                } else {
                    checked.2 += 1;
                }
                let mut h = Game::new(Screen::headless(), Rng::new(seed + n as u64));
                h.restore(campaign, tank, battle).unwrap();
                // what was saved comes back the same
                let again = crate::save::to_string(&h.save_info(), &h.campaign, &h.tank, &h.battle).unwrap();
                assert_eq!(&again, text, "seed {seed} save {n} changed on loading");
                h.resume_loaded();
                assert!(h.campaign.over, "seed {seed} save {n}: campaign left unfinished");
            }
        }
        eprintln!("loaded and played on: {} in battle, {} during a day, {} on the calendar", checked.0, checked.1, checked.2);
        assert!(checked.0 > 0 && checked.1 > 0 && checked.2 > 0, "{checked:?}");
    }

    #[test]
    fn half_blocks_pair_rows_and_keep_text() {
        let mut src = Console::new(2, 4);
        src.put_char_ex(0, 0, 219, color::RED, color::BLACK);
        src.put_char_ex(0, 1, 219, color::BLUE, color::BLACK);
        src.put_char_ex(1, 2, b'P', color::WHITE, color::GREY);
        src.put_char_ex(1, 3, 219, color::RED, color::BLACK);
        let out = scaled_blocks(&src, 2, 2);
        assert_eq!((out.w, out.h), (2, 2));
        let c = out.cell(0, 0).unwrap();
        assert_eq!((c.ch, c.fg, c.bg), (223, color::RED, color::BLUE));
        let c = out.cell(1, 1).unwrap();
        assert_eq!((c.ch, c.bg), (b'P', color::GREY));
        // twice the size: each cell becomes two across and two pixels down
        let big = scaled_blocks(&src, 4, 4);
        let c = big.cell(1, 0).unwrap();
        assert_eq!((c.fg, c.bg), (color::RED, color::RED));
        assert_eq!(big.cell(2, 2).unwrap().ch, b'P');
    }

    #[test]
    fn the_overview_map_grows_with_the_screen() {
        let src = crate::xp::by_name("pattons_best_map.xp").unwrap();
        assert_eq!(cal_map_size(&src, 100, 30), (85, 25));
        let (w, h) = cal_map_size(&src, 174, 49);
        assert!(w > 85 && h > 25 && h <= 49 - 4 - CAL_MAP_Y, "{w}x{h}");
    }

    #[test]
    fn the_overview_maps_fit_their_window() {
        for (file, xml) in defs::CAMPAIGN_FILES {
            let def = defs::load_campaign(xml).unwrap();
            let Some(name) = def.map_file else { continue };
            let src = crate::xp::by_name(&name).unwrap();
            let map = scaled_blocks(&src, src.w, (src.h + 1) / 2);
            assert!(map.h <= MIN_HEIGHT - CAL_MAP_Y - 3, "{file}");
            for d in &def.days {
                if let (Some(x), Some(y)) = (&d.map_x, &d.map_y) {
                    let (x, y): (i32, i32) = (x.trim().parse().unwrap(), y.trim().parse().unwrap());
                    assert!(x < map.w && y / 2 < map.h, "{file}: marker at {x},{y}");
                }
            }
        }
    }
}
