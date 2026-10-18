//! Windows not ported yet, and the campaign day screen's drawing, which
//! the encounter code calls. Each says so when opened.

use crate::color;
use crate::console::Align;
use crate::defs;
use crate::game::*;
use crate::layout::*;
use crate::ui::{Vk, wrap};

impl Game {
    fn not_ported(&mut self, what: &str) {
        self.popup(&format!("{what} is not ported yet."));
    }

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

    /// `ShowTankInfo`
    pub fn show_tank_info(&mut self, _select_tank: bool) {
        self.not_ported("Tank Info");
    }

    /// `ShowCrewInfo`
    pub fn show_crew_info(&mut self) {
        self.not_ported("Crew Info");
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

    /// `ShowCampaignStats`
    pub fn show_campaign_stats(&mut self) {
        self.not_ported("Campaign Stats");
    }

    /// `MainGunAmmoMenu`
    pub fn main_gun_ammo_menu(&mut self, _no_dark: bool) {
        self.not_ported("The main gun ammunition menu");
    }

    /// `RenderCampaign`
    pub fn render_campaign(&mut self, _no_flush: bool) {}

    /// `UpdateCActionCon`
    pub fn update_c_action_con(&mut self) {}

    /// `UpdateCInfoCon`
    pub fn update_c_info_con(&mut self, _mx: i32, _my: i32) {}

    /// `UpdateCOverlay`
    pub fn update_c_overlay(&mut self, _highlight_node: Option<usize>) {}
}
