//! `DisplayCredits`: the credits scrolling up the screen, one line every
//! 0.75 s, P to pause.

use crate::color;
use crate::console::{Align, COLCTRL_1, Console};
use crate::defs;
use crate::game::{KEY_HIGHLIGHT_COLOR, MENU_TITLE_COLOR, hl};
/// The credits and high scores are main menu screens, 100x30.
const SCREEN_WIDTH: i32 = crate::layout::MIN_WIDTH;
const SCREEN_HEIGHT: i32 = crate::layout::MIN_HEIGHT;
const SCREEN_XM: i32 = SCREEN_WIDTH / 2;
use std::time::{Duration, Instant};

const STEP: Duration = Duration::from_millis(750);
/// Rows the text scrolls through, between the title and the keys.
const TOP: i32 = 5;
const BOTTOM: i32 = SCREEN_HEIGHT - 6;

pub struct Credits {
    pub con: Console,
    current_line: i32,
    paused: bool,
    last: Instant,
}

impl Credits {
    pub fn new() -> Self {
        let mut con = Console::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        con.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);
        let mut c = Credits { con, current_line: 0, paused: false, last: Instant::now() };
        c.draw();
        c
    }

    fn draw(&mut self) {
        let c = &mut self.con;
        c.clear();
        c.fg = color::WHITE;
        c.frame(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, false, false);
        c.fg = MENU_TITLE_COLOR;
        c.print_ex(SCREEN_XM, 2, Align::Center, "-- Credits --");
        c.fg = color::WHITE;
        for (n, line) in defs::defs().credits_text.iter().enumerate() {
            let y = BOTTOM - (self.current_line - n as i32);
            if y < TOP {
                continue;
            }
            if y > BOTTOM {
                break;
            }
            c.print_ex(SCREEN_XM, y, Align::Center, line);
        }
        c.print_ex(SCREEN_XM, SCREEN_HEIGHT - 4, Align::Center, &format!("[{}] to Pause", hl("P")));
        c.print_ex(SCREEN_XM, SCREEN_HEIGHT - 3, Align::Center, &format!("[{}] to Return", hl("Enter or ESC")));
    }

    /// Scroll on if it is time; true when the screen changed.
    pub fn tick(&mut self) -> bool {
        if self.paused || self.last.elapsed() < STEP {
            return false;
        }
        self.last = Instant::now();
        self.current_line += 1;
        let len = defs::defs().credits_text.len() as i32;
        if self.current_line >= len + (BOTTOM - TOP) + 1 {
            self.current_line = 0;
        }
        self.draw();
        true
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
}

/// `DisplayHighScores`: the bones file's score list. The original's 40
/// rows do not fit in 30, so the best that fit are shown.
pub fn high_scores() -> Console {
    let bones = crate::bones::load();
    let mut c = Console::new(SCREEN_WIDTH, SCREEN_HEIGHT);
    c.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);
    c.frame(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT, false, false);
    c.fg = MENU_TITLE_COLOR;
    c.print_ex(SCREEN_XM, 2, Align::Center, "-- High Scores --");
    c.fg = color::WHITE;
    let cols = [2, 17, 35, 54, 65];
    for (x, title) in cols.iter().zip(["Campaign", "Tank", "Commander", "VP Score", "Outcome"]) {
        c.print(*x, 4, title);
    }
    c.hline(2, 5, SCREEN_WIDTH - 4);
    let mut y = 6;
    for s in &bones.score_list {
        if y > SCREEN_HEIGHT - 5 {
            break;
        }
        let mut vp = s.vp.to_string();
        if s.unlimited_tank_selection || s.casual_commander {
            vp.push_str(" (");
            if s.unlimited_tank_selection {
                vp.push('U');
            }
            if s.casual_commander {
                vp.push('C');
            }
            vp.push(')');
        }
        for (x, text) in cols.iter().zip([&s.campaign_name, &s.tank_name, &s.commander_name, &vp, &s.outcome]) {
            c.print(*x, y, text);
        }
        y += 1;
    }
    c.print_ex(SCREEN_XM, SCREEN_HEIGHT - 3, Align::Center, &format!("[{}] Continue", hl("Enter")));
    c
}
