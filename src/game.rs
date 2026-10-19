//! The original's globals (`campaign`, `tank`, `battle`, the consoles,
//! `key` and `mouse`) as one `Game`, and its general functions: dice,
//! messages, pop-ups, labels and the loops that wait for a key.

use crate::color::{self, Color};
use crate::console::{Align, COLCTRL_1, COLCTRL_STOP, Console};
use crate::defs;
use crate::layout::*;
use crate::model::*;
use crate::rng::Rng;
use crate::ui::{Screen, Vk, wrap};

pub const NAME: &str = "Armoured Commander";
pub const VERSION: &str = "1.0";
pub const SUBVERSION: &str = "9";

/// Veteran = 1 (normal), Regular = 2, Recruit = 3.
pub const DIFFICULTY: i32 = 1;
pub const BASE_EXP_REQ: i32 = 30 / DIFFICULTY;
pub const LVL_INFLATION: i32 = 10 / DIFFICULTY;

pub const EXTRA_AMMO: i32 = 30;
pub const NAME_MAX_LEN: usize = 17;
pub const NICKNAME_MAX_LEN: usize = 15;
pub const MAX_HS: usize = 40;

pub const STONE_ROAD_MOVE_TIME: i32 = 30;
pub const DIRT_ROAD_MOVE_TIME: i32 = 45;
pub const NO_ROAD_MOVE_TIME: i32 = 60;
pub const GROUND_MOVE_TIME_MODIFIER: i32 = 15;

pub const KEY_COLOR: Color = Color(255, 0, 255);
pub const MAP_B_COLOR: Color = Color(100, 120, 100);
pub const MAP_D_COLOR: Color = Color(70, 90, 70);
pub const OPEN_GROUND_COLOR: Color = Color(100, 140, 100);
pub const MUD_COLOR: Color = Color(80, 50, 30);
pub const HEX_EDGE_COLOR: Color = Color(60, 100, 60);
pub const ROAD_COLOR: Color = Color(160, 140, 100);
pub const DIRT_COLOR: Color = Color(80, 50, 30);
pub const CLEAR_SKY_COLOR: Color = Color(16, 180, 240);
pub const OVERCAST_COLOR: Color = Color(150, 150, 150);
pub const STONE_ROAD_COLOR: Color = color::DARKER_GREY;
pub const FRONTLINE_COLOR: Color = color::RED;
pub const PLAYER_COLOR: Color = Color(10, 64, 10);
pub const ENEMY_COLOR: Color = Color(80, 80, 80);
pub const ROW_COLOR: Color = Color(30, 30, 30);
pub const ROW_COLOR2: Color = Color(20, 20, 20);
pub const SELECTED_COLOR: Color = color::BLUE;
pub const HIGHLIGHT_COLOR: Color = color::LIGHT_BLUE;
pub const GREYED_COLOR: Color = Color(60, 60, 60);
pub const SKILL_ACTIVATE_COLOR: Color = Color(0, 255, 255);
pub const MENU_TITLE_COLOR: Color = color::LIGHTER_BLUE;
pub const KEY_HIGHLIGHT_COLOR: Color = Color(0, 200, 255);

/// `HIGHLIGHT`: `'[%cQ%c]uit' % HIGHLIGHT` in the original.
pub const HL_ON: char = '\u{1}';
pub const HL_OFF: char = '\u{8}';

/// Wrap a command key in the highlight colour controls.
pub fn hl(text: &str) -> String {
    format!("{HL_ON}{text}{HL_OFF}")
}

pub struct Game {
    pub rng: Rng,
    pub scr: Screen,
    /// libtcod's root console, console 0.
    pub root: Console,
    pub con: Console,
    pub map_con: Console,
    pub overlay_con: Console,
    pub map_info_con: Console,
    pub msg_con: Console,
    pub tank_con: Console,
    pub date_con: Console,
    pub menu_con: Console,
    pub text_con: Console,
    pub c_map_con: Console,
    pub c_overlay_con: Console,
    pub c_action_con: Console,
    pub c_info_con: Console,

    pub campaign: Campaign,
    pub tank: PlayerTank,
    pub battle: Option<Battle>,

    /// Every save, kept in memory for tests that load them again.
    pub test_saves: Option<Vec<String>>,
}

/// `CreateConsole`
pub fn create_console(w: i32, h: i32, bc: Color, fc: Color, a: Align) -> Console {
    let mut c = Console::new(w, h);
    c.bg = bc;
    c.fg = fc;
    c.align = a;
    c.clear();
    c.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);
    c
}

impl Game {
    pub fn new(scr: Screen, rng: Rng) -> Self {
        let (w, h) = scr.size();
        set_screen_size(w, h);
        let mut root = create_console(sw(), sh(), color::BLACK, color::WHITE, Align::Left);
        root.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);
        let mut overlay_con = create_console(map_con_width(), map_con_height(), KEY_COLOR, color::BLACK, Align::Left);
        overlay_con.key = Some(KEY_COLOR);
        let mut c_overlay_con = create_console(C_MAP_MIN_WIDTH, C_MAP_CON_HEIGHT, KEY_COLOR, color::WHITE, Align::Left);
        c_overlay_con.key = Some(KEY_COLOR);
        Game {
            rng,
            scr,
            root,
            con: create_console(sw(), sh(), color::BLACK, color::WHITE, Align::Left),
            map_con: create_console(map_con_width(), map_con_height(), color::BLACK, color::BLACK, Align::Left),
            overlay_con,
            map_info_con: create_console(30, 8, color::BLACK, color::WHITE, Align::Center),
            msg_con: create_console(MSG_CON_WIDTH, msg_con_height(), color::BLACK, color::WHITE, Align::Left),
            tank_con: create_console(TANK_CON_WIDTH, TANK_CON_HEIGHT, color::BLACK, color::WHITE, Align::Left),
            date_con: create_console(DATE_CON_WIDTH, DATE_CON_HEIGHT, color::BLACK, color::WHITE, Align::Left),
            menu_con: create_console(MENU_CON_WIDTH, MENU_CON_HEIGHT, color::BLACK, color::WHITE, Align::Left),
            text_con: create_console(TEXT_CON_WIDTH, TEXT_CON_HEIGHT, color::BLACK, color::WHITE, Align::Left),
            c_map_con: create_console(C_MAP_MIN_WIDTH, C_MAP_CON_HEIGHT, color::BLACK, color::BLACK, Align::Left),
            c_overlay_con,
            c_action_con: create_console(C_ACTION_CON_W, C_ACTION_CON_H, color::BLACK, color::WHITE, Align::Left),
            c_info_con: create_console(C_INFO_CON_W, C_INFO_CON_H, color::BLACK, color::WHITE, Align::Left),
            campaign: Campaign::new(),
            tank: PlayerTank::new(None),
            battle: None,
            test_saves: None,
        }
    }

    // ---------------------------------------------------------------------
    // the battle, its units and the crew, by index

    pub fn bt(&self) -> &Battle {
        self.battle.as_ref().expect("in a battle")
    }

    pub fn btm(&mut self) -> &mut Battle {
        self.battle.as_mut().expect("in a battle")
    }

    pub fn in_battle(&self) -> bool {
        self.battle.is_some()
    }

    pub fn u(&self, i: usize) -> &EnemyUnit {
        &self.bt().enemy_units[i]
    }

    pub fn um(&mut self, i: usize) -> &mut EnemyUnit {
        &mut self.btm().enemy_units[i]
    }

    /// The hex a unit is in.
    pub fn uhex(&self, i: usize) -> &MapHex {
        let b = self.bt();
        &b.maphexes[b.enemy_units[i].map_hex]
    }

    pub fn crew(&self, i: usize) -> &Crewman {
        &self.tank.crew[i]
    }

    pub fn crewm(&mut self, i: usize) -> &mut Crewman {
        &mut self.tank.crew[i]
    }

    /// `GetCrewByPosition`
    pub fn crew_pos(&self, position: &str) -> Option<usize> {
        self.tank.crew_by_position(position)
    }

    // ---------------------------------------------------------------------
    // screen and input

    /// `console_flush`
    pub fn flush(&mut self) {
        self.scr.flush(&self.root);
    }

    /// `sys_check_for_event`
    pub fn check_for_event(&mut self) {
        self.scr.check_for_event();
        if let Some((w, h)) = self.scr.resized.take() {
            self.resize(w, h);
        }
    }

    /// Fit the battle to the hex size in use, after a resize or loading
    /// a game saved at another size: the hexes' places, the map consoles,
    /// and each unit at a new spot inside its own hex. Nothing changes if
    /// the hexes are already where they belong.
    pub fn rescale_battle(&mut self) {
        let Some(b) = &self.battle else { return };
        if b.maphexes.iter().all(|h| (h.x, h.y) == hex2screen(h.hx, h.hy)) && self.map_con.w == map_con_width() {
            return;
        }
        log::info!("encounter map: hex size {}, {}x{}", hex_k(), map_con_width(), map_con_height());
        for h in &mut self.btm().maphexes {
            (h.x, h.y) = hex2screen(h.hx, h.hy);
        }
        for i in 0..self.bt().enemy_units.len() {
            if self.u(i).alive {
                let (x, y) = self.unit_char_location(i);
                let u = self.um(i);
                u.x = x;
                u.y = y;
            }
        }
        self.map_con = create_console(map_con_width(), map_con_height(), color::BLACK, color::BLACK, Align::Left);
        self.overlay_con = create_console(map_con_width(), map_con_height(), KEY_COLOR, color::BLACK, Align::Left);
        self.overlay_con.key = Some(KEY_COLOR);
        self.paint_map_con();
        self.update_map_overlay(false);
    }

    /// The terminal changed size: lay the screen out again and redraw the
    /// encounter or day map under way. A window open at the time redraws
    /// itself at its next key.
    pub fn resize(&mut self, w: i32, h: i32) {
        set_screen_size(w, h);
        log::info!("resize: terminal {w}x{h}, screen {}x{}", sw(), sh());
        self.root = create_console(sw(), sh(), color::BLACK, color::WHITE, Align::Left);
        self.root.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);
        self.con = create_console(sw(), sh(), color::BLACK, color::WHITE, Align::Left);
        self.msg_con = create_console(MSG_CON_WIDTH, msg_con_height(), color::BLACK, color::WHITE, Align::Left);
        if self.battle.is_some() {
            self.rescale_battle();
            self.update_msg_con();
            self.render_encounter(false);
        } else if self.campaign.day_in_progress && self.campaign.day_map.is_some() {
            let p = self.campaign.day_map().player_node;
            self.move_view_to(Some(p));
        }
    }

    /// `Wait`: check for an event (so the terminal stays responsive) and
    /// sleep. Headless runs do not sleep.
    pub fn wait(&mut self, ms: u64) {
        self.check_for_event();
        if !self.scr.is_headless() {
            std::thread::sleep(std::time::Duration::from_millis(ms));
        }
    }

    /// `console_blit(con, 0, 0, sw(), sh(), 0, 0, 0)`
    pub fn blit_con(&mut self) {
        Console::blit(&self.con, 0, 0, sw(), sh(), &mut self.root, 0, 0);
    }

    /// Clear `con` and darken the screen behind a window, as the original
    /// does before every window.
    pub fn darken_screen(&mut self) {
        self.con.clear();
        self.root.darken(0.7);
    }

    /// `console_blit(menu_con, ..., 0, menu_con_x(), menu_con_y())`
    pub fn blit_menu(&mut self) {
        Console::blit(&self.menu_con, 0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, &mut self.root, menu_con_x(), menu_con_y());
    }

    /// The character of the last key, `chr(key.c)`.
    pub fn key_char(&self) -> char {
        self.scr.key.c
    }

    pub fn key_vk(&self) -> Vk {
        self.scr.key.vk
    }

    // ---------------------------------------------------------------------
    // dice

    /// `Roll1D100`
    pub fn roll_1d100(&mut self) -> i32 {
        self.rng.int(1, 100)
    }

    /// `Roll1D10`
    pub fn roll_1d10(&mut self) -> i32 {
        self.rng.int(1, 10)
    }

    /// `Roll1D6`
    pub fn roll_1d6(&mut self) -> i32 {
        self.rng.int(1, 6)
    }

    /// `Roll2D6`: (d1, d2, total)
    pub fn roll_2d6(&mut self) -> (i32, i32, i32) {
        let d1 = self.rng.int(1, 6);
        let d2 = self.rng.int(1, 6);
        (d1, d2, d1 + d2)
    }

    // ---------------------------------------------------------------------
    // journal, messages, experience

    /// `WriteJournal`
    pub fn write_journal(&mut self, text: &str) {
        log::info!("journal: {text}");
        self.campaign.campaign_journal.push(text.to_string());
    }

    /// `GetExpReq`
    pub fn get_exp_req(level: i32) -> i32 {
        let mut x = (level - 1) * BASE_EXP_REQ;
        if level > 2 {
            for l in 3..=level {
                x += LVL_INFLATION * (l - 2);
            }
        }
        x
    }

    /// `Message`: add to the encounter message log (nothing outside an
    /// encounter).
    pub fn message(&mut self, new_msg: &str) {
        self.message_col(new_msg, color::WHITE);
    }

    pub fn message_col(&mut self, new_msg: &str, col: Color) {
        if self.battle.is_none() {
            return;
        }
        log::info!("message: {new_msg}");
        let lines = wrap(new_msg, (MSG_CON_WIDTH - 1) as usize, " ");
        let cap = self.msg_capacity();
        let b = self.btm();
        for line in lines {
            if b.messages.len() >= cap {
                b.messages.remove(0);
            }
            b.messages.push((line, col));
        }
        self.update_msg_con();
        self.render_encounter(false);
    }

    /// How many message lines the log keeps: the most the panel can show.
    pub fn msg_capacity(&self) -> usize {
        100
    }

    /// `PopUp`: a window with a message; with `confirm`, y or n. Returns
    /// whether the player said yes.
    pub fn popup_ex(&mut self, message: &str, confirm: bool, skip_update: bool) -> bool {
        log::info!("popup: {message}");
        self.darken_screen();
        let lines = wrap(message, 30, "");
        let h = lines.len() as i32 + 6;
        let mut y = (sh() - h) / 2 + 2;

        self.root.fg = color::WHITE;
        self.root.bg = color::BLACK;
        self.root.frame(sxm() - 17, y - 2, 34, h, true, true);
        for line in &lines {
            self.root.print_ex(sxm(), y, Align::Center, line);
            y += 1;
        }
        let text = if confirm {
            format!("[{}] or [{}]", hl("y"), hl("N"))
        } else {
            format!("[{}] to continue", hl("Enter"))
        };
        self.root.print_ex(sxm(), y + 1, Align::Center, &text);

        let mut choice = false;
        loop {
            self.check_for_event();
            let kc = self.key_char();
            let vk = self.key_vk();
            if confirm {
                if vk == Vk::Enter || vk == Vk::Escape || kc == 'n' || kc == 'N' {
                    break;
                } else if kc == 'y' || kc == 'Y' {
                    choice = true;
                    break;
                }
            } else if vk == Vk::Enter {
                break;
            }
            if self.scr.is_headless() {
                break;
            }
            self.flush();
        }

        if skip_update {
            return choice;
        }
        if self.battle.is_some() {
            self.render_encounter(true);
        } else if self.campaign.day_in_progress {
            self.render_campaign(true);
        }
        choice
    }

    pub fn popup(&mut self, message: &str) {
        self.popup_ex(message, false, false);
    }

    pub fn popup_confirm(&mut self, message: &str) -> bool {
        self.popup_ex(message, true, false)
    }

    /// `ShowLabel`: a grey label revealed under a point on the map, then a
    /// wait for Enter (or a pause, by the settings); in an encounter the
    /// text goes to the message log too.
    pub fn show_label(&mut self, x: i32, y: i32, original_text: &str, crewman: Option<usize>) {
        let mut x = x;
        let mut text = String::new();
        if let Some(c) = crewman {
            text.push_str(&self.tank.crew[c].position);
            text.push_str(": ");
        }
        text.push_str(original_text);
        if self.campaign.pause_labels {
            text.push_str(" [Enter to continue]");
        }
        log::info!("label: {original_text}");

        let lines = wrap(&text, 28, "");
        self.root.bg = GREYED_COLOR;
        self.root.fg = color::WHITE;
        let min_x = if self.battle.is_some() { map_con_x() } else { C_MAP_CON_X };
        let mut n = 1;
        for line in &lines {
            if y + n >= sh() {
                break;
            }
            let half = (line.chars().count() as i32 + 1) / 2;
            if x + half >= sw() {
                x = sw() - half;
            }
            if x - half <= min_x {
                x = min_x + half;
            }
            if !self.campaign.animations || self.scr.is_headless() {
                self.root.print_ex_bg(x, y + n, Align::Center, line);
                self.flush();
            } else {
                let chars: Vec<char> = line.chars().collect();
                let mut i = 0;
                while i <= chars.len() {
                    let part: String = chars[..i].iter().collect();
                    self.root.print_ex_bg(x, y + n, Align::Center, &part);
                    self.flush();
                    i += 2;
                }
                self.root.print_ex_bg(x, y + n, Align::Center, line);
                self.flush();
            }
            n += 1;
        }

        if self.campaign.pause_labels {
            self.wait_for_enter();
        } else {
            self.wait(1100);
        }
        self.root.bg = color::BLACK;

        if self.battle.is_some() {
            self.message_col(original_text, color::LIGHT_GREY);
            self.render_encounter(false);
        }
    }

    /// The keys every wait loop answers besides its own: F7 or 7 toggles
    /// sound. (F6, the screenshot, has no terminal equivalent.)
    fn common_keys(&mut self) {
        if matches!(self.key_vk(), Vk::F(7) | Vk::Digit(7)) {
            self.campaign.sounds = !self.campaign.sounds;
            let text = if self.campaign.sounds { "Sound turned on" } else { "Sound turned off" };
            self.popup(text);
        }
    }

    /// `WaitForEnter`
    pub fn wait_for_enter(&mut self) {
        self.wait_for(Vk::Enter);
    }

    /// `WaitForSpace`
    pub fn wait_for_space(&mut self) {
        self.wait_for(Vk::Space);
    }

    /// `WaitForEscape`
    pub fn wait_for_escape(&mut self) {
        self.wait_for(Vk::Escape);
    }

    fn wait_for(&mut self, vk: Vk) {
        loop {
            self.check_for_event();
            if self.key_vk() == vk || self.scr.is_headless() {
                break;
            }
            self.common_keys();
            self.flush();
        }
    }

    /// `CrewTalk`: a crewman (never the commander) says something.
    pub fn crew_talk(&mut self, message: &str, position_list: Option<&[&str]>) {
        let crew_list: Vec<usize> = (0..self.tank.crew.len())
            .filter(|&i| {
                let c = &self.tank.crew[i];
                c.position != "Commander"
                    && !c.no_actions()
                    && position_list.is_none_or(|p| p.contains(&c.position.as_str()))
            })
            .collect();
        if crew_list.is_empty() {
            return;
        }
        let crewman = *self.rng.choice(&crew_list);
        let (x, y) = if self.battle.is_none() {
            let (x, y) = self.node_screen_pos(self.campaign.day_map().player_node);
            (x, y - 1)
        } else {
            (map_x0() + map_con_x(), map_y0() + map_con_y())
        };
        self.show_label(x, y, message, Some(crewman));
        self.blit_con();
    }

    /// `GetChoice`: pick one of several options, or Escape for none.
    pub fn get_choice(&mut self, prompt_text: &str, choice_list: &[String]) -> Option<String> {
        if self.battle.is_some() {
            self.darken_screen();
        }
        let mut selected = 0usize;
        let mut result = Some(());
        loop {
            let m = &mut self.menu_con;
            m.align = Align::Center;
            m.clear();
            m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
            m.print(MENU_CON_XM, 3, prompt_text);
            let mut y = 6;
            for (i, choice) in choice_list.iter().enumerate() {
                if i == selected {
                    m.bg = SELECTED_COLOR;
                    let w = choice.len() as i32 + 2;
                    m.rect(MENU_CON_XM - w / 2, y, w, 1, false);
                    m.bg = color::BLACK;
                }
                m.print(MENU_CON_XM, y, choice);
                y += 2;
            }
            m.print(MENU_CON_XM, MENU_CON_HEIGHT - 4, &format!("[{}] to move selection", hl("W/S/Up/Down")));
            m.print(MENU_CON_XM, MENU_CON_HEIGHT - 3, "[Enter] to choose, [ESC] to cancel");
            self.blit_menu();
            self.flush();

            if self.scr.is_headless() {
                break;
            }
            let mut refresh = false;
            let mut exit = false;
            while !refresh {
                self.check_for_event();
                let vk = self.key_vk();
                let kc = self.key_char();
                if vk == Vk::Enter {
                    refresh = true;
                    exit = true;
                } else if vk == Vk::Escape {
                    result = None;
                    refresh = true;
                    exit = true;
                }
                if kc == 'w' || kc == 'W' || vk == Vk::Up {
                    selected = if selected == 0 { choice_list.len() - 1 } else { selected - 1 };
                    refresh = true;
                } else if kc == 's' || kc == 'S' || vk == Vk::Down {
                    selected = if selected == choice_list.len() - 1 { 0 } else { selected + 1 };
                    refresh = true;
                }
                self.flush();
            }
            if exit {
                break;
            }
        }
        self.menu_con.align = Align::Center;
        result.map(|_| choice_list[selected].clone())
    }

    /// `GetInput`: type a line of text into `con`, centred on the screen,
    /// Ctrl+R for a random entry.
    pub fn get_input(&mut self, prompt_text: &str, y: i32, max_length: usize, random_list: &[String], get_name: bool) -> String {
        let mut input_text = String::new();
        let x = sxm() - max_length as i32 / 2;
        let w = 84.min(sw() - 2);
        self.con.frame(sxm() - w / 2, y - 5, w, 13, true, false);
        loop {
            let c = &mut self.con;
            c.print_ex(sxm(), y - 3, Align::Center, prompt_text);
            c.bg = PLAYER_COLOR;
            c.rect(x, y, max_length as i32, 1, true);
            c.print_ex(sxm(), y, Align::Center, &input_text);
            c.bg = color::BLACK;
            if !random_list.is_empty() || get_name {
                c.print_ex(sxm(), y + 4, Align::Center, "Press Ctrl+R to randomly select an entry, replacing anything already inputted");
            }
            c.print_ex(sxm(), y + 5, Align::Center, &format!("[{}] to continue", hl("Enter")));
            self.blit_con();
            self.flush();
            if self.scr.is_headless() {
                break;
            }

            let mut refresh = false;
            let mut exit = false;
            while !refresh {
                self.check_for_event();
                let key = self.scr.key;
                if key.vk == Vk::Enter {
                    refresh = true;
                    exit = true;
                } else if key.vk == Vk::Backspace {
                    if input_text.pop().is_some() {
                        refresh = true;
                    }
                }
                let ch = key.c;
                if (' '..='~').contains(&ch) {
                    if ch == 'r' && key.ctrl {
                        if get_name {
                            let d = defs::defs();
                            for _ in 0..99 {
                                input_text = format!("{} {}", self.rng.choice(&d.first_names), self.rng.choice(&d.last_names));
                                if input_text.len() <= max_length {
                                    break;
                                }
                            }
                            refresh = true;
                        } else if !random_list.is_empty() {
                            for _ in 0..99 {
                                let mut s = self.rng.choice(random_list).clone();
                                s.truncate(max_length);
                                if s != input_text {
                                    input_text = s;
                                    break;
                                }
                            }
                            refresh = true;
                        }
                    } else if input_text.len() < max_length {
                        input_text.push(ch);
                        refresh = true;
                    }
                }
                self.flush();
            }
            if exit {
                break;
            }
        }
        input_text
    }

    /// `TutorialMessage`: shown once per player, unless turned off. The
    /// bones file that remembers which were shown is not ported yet, so
    /// this keeps them for the session.
    pub fn tutorial_message(&mut self, key: &str) {
        if !self.campaign.tutorial_message {
            return;
        }
        let Some(text) = defs::defs().tutorial_text.get(key).cloned() else { return };
        if !crate::bones::mark_tutorial(key) {
            return;
        }
        if !self.scr.is_headless() {
            let mut w = 3;
            while w < MENU_CON_WIDTH {
                let h = (w / 3 - 3).max(3);
                self.root.rect(sxm() - w / 2, sym() - h / 2, w, h, false);
                self.root.frame(sxm() - w / 2, sym() - h / 2, w, h, true, false);
                self.flush();
                w += 6;
            }
        }
        let m = &mut self.menu_con;
        m.clear();
        m.align = Align::Left;
        m.frame(0, 0, MENU_CON_WIDTH, MENU_CON_HEIGHT, false, false);
        m.fg = MENU_TITLE_COLOR;
        m.print_ex(MENU_CON_XM, 1, Align::Center, "Tutorial Message");
        m.fg = color::WHITE;
        let lines = wrap(&text, 60, " ");
        let mut y = MENU_CON_HEIGHT / 2 - lines.len() as i32 / 2;
        for line in &lines {
            m.print(MENU_CON_XM - 30, y, line);
            y += 1;
        }
        m.print_ex(MENU_CON_XM, MENU_CON_HEIGHT - 2, Align::Center, &format!("[{}] to Continue", hl("Enter")));
        self.blit_menu();
        self.flush();
        self.wait_for_enter();
        self.blit_con();
        self.flush();
    }
}

/// Text with its colour controls removed, for logs and tests.
pub fn plain(text: &str) -> String {
    text.chars().filter(|&c| c != HL_ON && c != HL_OFF && !('\u{2}'..='\u{5}').contains(&c)).collect()
}

#[allow(dead_code)]
const _: () = assert!(COLCTRL_1 == 1 && COLCTRL_STOP == 8);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_requirements() {
        assert_eq!(Game::get_exp_req(1), 0);
        assert_eq!(Game::get_exp_req(2), 30);
        assert_eq!(Game::get_exp_req(3), 70);
        assert_eq!(Game::get_exp_req(4), 120);
    }
}
