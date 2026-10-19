//! The main menu (`MainMenu` in armcom.py): a sunset sky, clouds and hills
//! scrolling past the title, the menu, the poppy and a tombstone from the
//! bones file. Ported line for line, laid out for the terminal's size: the
//! 30-row scene is drawn in half blocks at the height there is room for
//! (15 rows at 100x30, its own 30 from 45 rows up), the text below it.

use crate::color::{self, Color};
use crate::console::{Align, COLCTRL_1, Console};
use crate::layout::{sh, sw};
use crate::rng::Rng;
use std::time::{Duration, Instant};

/// The original's screen width, beside which the tombstone stood 30
/// columns in from the right.
const SCREEN_WIDTH: i32 = 149;
/// The scene's height in the original, its rows as the clouds and ground
/// move in them.
const SCENE_H: i32 = 30;

const VERSION: &str = "1.0";
const SUBVERSION: &str = "9";
const WEBSITE: &str = "www.armouredcommander.com";
const GITHUB: &str = "github.com/sudasana/armcom";

const KEY_COLOR: Color = Color(255, 0, 255);
const KEY_HIGHLIGHT_COLOR: Color = Color(0, 200, 255);
const TITLE_GROUND_COLOR: Color = Color(26, 79, 5);

/// `HIGHLIGHT`: the pair of colour controls around a command key.
const HL_ON: char = '\u{1}';
const HL_OFF: char = '\u{8}';

const GROUND_EVERY: Duration = Duration::from_millis(150);
const CLOUD_EVERY: Duration = Duration::from_millis(300);

/// "ARMOURED COMMANDER" as runs of cells, alternately left alone and
/// filled black (`TITLE` in armcom_defs.py).
const TITLE: [&[i32]; 25] = [
    &[1, 6, 2, 6, 3, 3, 3, 3, 2, 6, 2, 3, 2, 3, 1, 6, 3, 6, 1, 6],
    &[0, 8, 1, 7, 2, 4, 1, 4, 1, 8, 1, 3, 2, 3, 1, 7, 2, 6, 1, 7],
    &[0, 8, 1, 8, 1, 9, 1, 8, 1, 3, 2, 3, 1, 8, 1, 6, 1, 8],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 9, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 3, 1, 1, 1, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 3, 2, 3, 1, 3, 2, 2, 2, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 2, 2, 4, 3, 3, 2, 3],
    &[0, 8, 1, 6, 3, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 6, 3, 4, 3, 3, 2, 3],
    &[0, 3, 2, 3, 1, 3, 2, 2, 2, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 2, 2, 3, 4, 3, 2, 3],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 3, 3, 3, 1, 8, 1, 8, 1, 3, 2, 3, 1, 6, 1, 8],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 3, 3, 3, 1, 8, 1, 8, 1, 3, 2, 3, 1, 6, 1, 7],
    &[0, 3, 2, 3, 1, 3, 2, 3, 1, 3, 3, 3, 2, 6, 3, 6, 2, 3, 2, 3, 1, 6, 1, 6],
    &[0],
    &[1, 7, 2, 6, 2, 3, 3, 3, 1, 3, 3, 3, 2, 6, 2, 3, 2, 3, 1, 6, 3, 6, 1, 6],
    &[0, 8, 1, 8, 1, 4, 1, 4, 1, 4, 1, 4, 1, 8, 1, 3, 2, 3, 1, 7, 2, 6, 1, 7],
    &[0, 8, 1, 8, 1, 9, 1, 9, 1, 8, 1, 3, 2, 3, 1, 8, 1, 6, 1, 8],
    &[0, 3, 6, 3, 2, 3, 1, 9, 1, 9, 1, 3, 2, 3, 1, 4, 1, 3, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 3, 6, 3, 2, 3, 1, 3, 1, 1, 1, 3, 1, 3, 1, 1, 1, 3, 1, 3, 2, 3, 1, 8, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 3, 6, 3, 2, 3, 1, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 8, 1, 3, 2, 3, 1, 4, 3, 3, 2, 2],
    &[0, 3, 6, 3, 2, 3, 1, 3, 3, 3, 1, 3, 3, 3, 1, 8, 1, 8, 1, 3, 2, 3, 1, 4, 3, 6],
    &[0, 3, 6, 3, 2, 3, 1, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 1, 4, 1, 3, 2, 3, 1, 3, 4, 3, 2, 2],
    &[0, 3, 6, 3, 2, 3, 1, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 3, 4, 3, 2, 3],
    &[0, 8, 1, 8, 1, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 8, 1, 6, 1, 3, 2, 3],
    &[0, 8, 1, 8, 1, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 7, 2, 6, 1, 3, 2, 3],
    &[1, 7, 2, 6, 2, 3, 3, 3, 1, 3, 3, 3, 1, 3, 2, 3, 1, 3, 2, 3, 1, 6, 3, 6, 1, 3, 2, 3],
];

pub enum Choice {
    Continue,
    NewCampaign,
    HighScores,
    Credits,
    Quit,
}

pub struct MainMenu {
    pub con: Console,
    /// The screen's size when the menu was made; the menu is made again
    /// when the terminal is resized.
    w: i32,
    h: i32,
    background: Console,
    title: Console,
    clouds: Console,
    ground: Console,
    /// The sky, clouds, ground and title put together, 30 rows tall as in
    /// the original, before it is drawn to the screen at its height.
    scene: Console,
    cloud_altitude: i32,
    cloud_height: i32,
    cloud_length: i32,
    c_cloud_length: i32,
    ground_height: i32,
    ground_click: Instant,
    cloud_click: Instant,
    /// The saved game's summary, or why it cannot be loaded.
    saved: Option<Result<crate::save::Info, String>>,
    /// New Campaign was pressed with a saved game: asking whether to
    /// replace it.
    confirm_new: bool,
    /// `GetRandomGrave`: a fallen crewman from the bones file's graveyard.
    tombstone: Option<[String; 5]>,
}

/// Rows of text under the scene: the tagline, the saved game, the four
/// choices, and the copyright lines.
const TEXT_ROWS: i32 = 15;

impl MainMenu {
    /// The menu for a screen of the size `layout` holds.
    pub fn new(rng: &mut Rng) -> Self {
        let (w, h) = (sw(), sh());

        // the sunset sky
        let mut background = Console::new(w, SCENE_H);
        let color1 = Color(255, 255, 114);
        let color2 = Color(193, 8, 3);
        for y in 0..SCENE_H {
            background.bg = color::lerp(color1, color2, y as f32 / SCENE_H as f32);
            background.rect(0, y, w, 1, false);
        }

        // "ARMOURED COMMANDER"
        let mut title = Console::new(80, 25);
        title.key = Some(KEY_COLOR);
        title.bg = KEY_COLOR;
        title.clear();
        title.bg = color::BLACK;
        for (y, line) in TITLE.iter().enumerate() {
            let mut x = 0;
            let mut black = false;
            for &width in line.iter() {
                if black {
                    title.rect(x, y as i32, width, 1, false);
                }
                black = !black;
                x += width;
            }
        }

        let mut clouds = Console::new(w, SCENE_H);
        clouds.bg = KEY_COLOR;
        clouds.clear();
        clouds.key = Some(KEY_COLOR);
        clouds.bg = color::WHITE;

        let mut ground = Console::new(w, SCENE_H);
        ground.bg = KEY_COLOR;
        ground.clear();
        ground.bg = TITLE_GROUND_COLOR;
        ground.key = Some(KEY_COLOR);

        let mut con = Console::new(w, h);
        con.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);

        let graveyard = crate::bones::load().graveyard;
        let tombstone = (!graveyard.is_empty()).then(|| rng.choice(&graveyard).clone());

        let mut menu = MainMenu {
            con,
            w,
            h,
            background,
            title,
            clouds,
            ground,
            scene: Console::new(w, SCENE_H),
            cloud_altitude: 0,
            cloud_height: 0,
            cloud_length: 0,
            c_cloud_length: 0,
            ground_height: 0,
            ground_click: Instant::now(),
            cloud_click: Instant::now(),
            saved: crate::save::peek(),
            confirm_new: false,
            tombstone,
        };
        for _ in 0..w {
            menu.update_cloud_layer(rng);
        }
        menu.ground_height = rng.int(3, 8);
        for x in 0..w {
            menu.ground.rect(x, SCENE_H - menu.ground_height, 1, menu.ground_height, false);
            menu.ground_height = new_ground_height(rng, menu.ground_height);
        }
        menu.draw_menu();
        menu.draw_layers();
        menu
    }

    /// `UpdateCloudLayer`: scroll the clouds one column left and draw the
    /// new right-hand column.
    fn update_cloud_layer(&mut self, rng: &mut Rng) {
        let w = self.w;
        let c = &mut self.clouds;
        c.key = Some(color::BLACK);
        c.blit_self(1, 0, w - 1, SCENE_H, 0, 0);
        c.key = Some(KEY_COLOR);
        c.bg = KEY_COLOR;
        c.rect(w - 1, 0, 1, SCENE_H, false);
        c.bg = color::WHITE;

        if self.c_cloud_length > 0 {
            let mut h = self.cloud_height;
            if self.c_cloud_length == self.cloud_length || self.c_cloud_length == 1 {
                h -= 2;
            }
            c.rect(w - 1, SCENE_H - self.cloud_altitude - h / 2, 1, h, false);
            self.c_cloud_length -= 1;
        } else if rng.int(1, 14) == 1 {
            self.cloud_altitude = rng.int(10, 29);
            self.cloud_height = *rng.choice(&[3, 5, 7]);
            self.cloud_length = rng.int(10, 17);
            self.c_cloud_length = self.cloud_length;
        }
    }

    fn scroll_ground(&mut self, rng: &mut Rng) {
        let w = self.w;
        let g = &mut self.ground;
        let gh = self.ground_height;
        g.key = Some(color::BLACK);
        g.blit_self(1, 0, w - 1, SCENE_H, 0, 0);
        g.key = Some(KEY_COLOR);
        g.bg = KEY_COLOR;
        g.rect(w - 1, 0, 1, SCENE_H - gh, false);
        g.bg = TITLE_GROUND_COLOR;
        g.rect(w - 1, SCENE_H - gh, 1, gh, false);
        self.ground_height = new_ground_height(rng, gh);
    }

    /// The scene's height on the screen: its 30 rows as in the original
    /// when there is room, and down to 15 (in half blocks) when not.
    fn scene_rows(&self) -> i32 {
        (self.h - TEXT_ROWS).clamp(SCENE_H / 2, SCENE_H)
    }

    /// `DrawLayers`: the scene, drawn at its height on the screen in half
    /// blocks, each half sampled from the nearest of the scene's rows.
    fn draw_layers(&mut self) {
        let w = self.w;
        let s = &mut self.scene;
        Console::blit(&self.background, 0, 0, w, SCENE_H, s, 0, 0);
        Console::blit(&self.clouds, 0, 0, w, SCENE_H, s, 0, 0);
        Console::blit(&self.ground, 0, 0, w, SCENE_H, s, 0, 0);
        Console::blit(&self.title, 0, 0, 80, 25, s, (w - 80) / 2, 2);

        let rows = self.scene_rows();
        for y in 0..rows {
            for x in 0..w {
                let upper = self.scene.char_bg(x, 2 * y * SCENE_H / (2 * rows));
                let lower = self.scene.char_bg(x, (2 * y + 1) * SCENE_H / (2 * rows));
                self.con.put_char_ex(x, y, 223, upper, lower);
            }
        }
        if self.confirm_new {
            self.draw_confirm();
        }
    }

    fn draw_menu(&mut self) {
        let (w, h) = (self.w, self.h);
        let xm = w / 2;
        let rows = self.scene_rows();
        // spare rows go between the parts, as the original spaces them
        let spare = h - rows - TEXT_ROWS;
        let gap = if spare >= 4 { 1 } else { 0 };
        let con = &mut self.con;
        con.bg = color::BLACK;
        con.clear();
        con.align = Align::Center;
        let mut y = rows + gap;
        con.print(xm, y, "The World War II Tank Commander Roguelike");
        y += 1 + gap;

        if let Some(saved) = &self.saved {
            con.print(xm, y, &format!("[{HL_ON}C{HL_OFF}]ontinue Campaign:"));
            y += 1;
            match saved {
                Ok(info) => {
                    con.fg = color::LIGHT_GREY;
                    for line in [&info.campaign_name, &info.commander_name, &info.tank_name, &info.current_date] {
                        con.print(xm, y, line);
                        y += 1;
                    }
                }
                Err(e) => {
                    con.fg = color::LIGHT_RED;
                    con.print(xm, y, "The saved game cannot be loaded:");
                    con.print(xm, y + 1, e);
                    y += 4;
                }
            }
            con.fg = color::WHITE;
            y += gap;
        }

        for text in ["[\u{1}N\u{8}]ew Campaign", "[\u{1}H\u{8}]igh Scores", "[\u{1}V\u{8}]iew Credits", "[\u{1}Q\u{8}]uit"] {
            con.print(xm, y, text);
            y += 1;
        }

        con.fg = color::LIGHT_GREY;
        con.print_ex(xm, h - 5, Align::Center, &format!("version {VERSION}{SUBVERSION}"));
        con.print_ex(xm, h - 4, Align::Center, "Copyright 2015-2017 Gregory Adam Scott");
        con.print_ex(xm, h - 3, Align::Center, "Free Software under the GNU General Public License");
        con.print_ex(xm, h - 2, Align::Center, WEBSITE);
        con.print_ex(xm, h - 1, Align::Center, GITHUB);

        // the poppy
        con.fg = color::RED;
        con.align = Align::Left;
        con.print(4, h - 6, ",--.");
        con.print(3, h - 5, ".\\  /.");
        con.print(2, h - 4, "(  ()  )");
        con.print(3, h - 3, "`/  \\'");
        con.print(4, h - 2, "`--'");

        con.fg = color::WHITE;
        con.align = Align::Center;
        let text = if is_armistice_day() { "NEVER AGAIN" } else { "In Memoriam" };
        con.print(16, h - 3, text);

        // the tombstone, bottom right; closer to the edge on a narrow
        // screen, clear of the copyright lines
        if let Some(stone) = &self.tombstone {
            let x0 = if w >= SCREEN_WIDTH { w - 30 } else { w - 21 };
            con.bg = color::DARK_GREY;
            con.fg = color::BLACK;
            con.rect(x0 + 2, h - 10, 16, 1, false);
            con.rect(x0 + 1, h - 9, 18, 1, false);
            con.rect(x0, h - 8, 20, 8, false);
            for (n, text) in stone.iter().enumerate() {
                let text: String = text.chars().take(20).collect();
                con.print_ex(x0 + 10, h - 8 + n as i32, Align::Center, &text);
            }
            con.bg = color::BLACK;
            con.fg = color::WHITE;
        }
    }

    /// The question before New Campaign replaces a saved game, in a box
    /// over the scene, as the original's pop-up.
    fn draw_confirm(&mut self) {
        let lines = [
            "Starting a new campaign will erase",
            "the currently saved one in progress.",
            "Are you sure?",
            "",
            "[\u{1}y\u{8}] or [\u{1}N\u{8}]",
        ];
        let (bw, bh) = (42, lines.len() as i32 + 4);
        let (x, y) = ((self.w - bw) / 2, (self.scene_rows() - bh) / 2);
        let con = &mut self.con;
        con.fg = color::WHITE;
        con.bg = color::BLACK;
        con.frame(x, y, bw, bh, true, true);
        for (n, line) in lines.iter().enumerate() {
            con.print_ex(x + bw / 2, y + 2 + n as i32, Align::Center, line);
        }
    }

    /// One pass of the menu loop's animation; true when something moved.
    pub fn tick(&mut self, rng: &mut Rng) -> bool {
        let mut moved = false;
        if self.ground_click.elapsed() >= GROUND_EVERY {
            self.scroll_ground(rng);
            self.ground_click = Instant::now();
            moved = true;
        }
        if self.cloud_click.elapsed() >= CLOUD_EVERY {
            self.update_cloud_layer(rng);
            self.cloud_click = Instant::now();
            moved = true;
        }
        if moved {
            self.draw_layers();
        }
        moved
    }

    pub fn key(&mut self, c: char) -> Option<Choice> {
        let c = c.to_ascii_lowercase();
        if self.confirm_new {
            self.confirm_new = false;
            self.draw_menu();
            self.draw_layers();
            return (c == 'y').then_some(Choice::NewCampaign);
        }
        match c {
            'c' if self.saved.as_ref().is_some_and(|s| s.is_ok()) => Some(Choice::Continue),
            'n' if self.saved.is_some() => {
                self.confirm_new = true;
                self.draw_layers();
                None
            }
            'n' => Some(Choice::NewCampaign),
            'h' => Some(Choice::HighScores),
            'v' => Some(Choice::Credits),
            'q' => Some(Choice::Quit),
            _ => None,
        }
    }
}

/// `GetNewGroundHeight`.
fn new_ground_height(rng: &mut Rng, current: i32) -> i32 {
    match rng.int(1, 10) {
        1 if current > 3 => current - 1,
        10 if current < 8 => current + 1,
        _ => current,
    }
}

/// 11 November, by the clock's UTC date (the original used local time).
fn is_armistice_day() -> bool {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (_, month, day) = civil_from_days(secs.div_euclid(86_400));
    month == 11 && day == 11
}

/// Days since 1970-01-01 to (year, month, day), Howard Hinnant's algorithm.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        // 2026-11-11
        assert_eq!(civil_from_days(20_768), (2026, 11, 11));
    }

    #[test]
    fn title_rows_fit() {
        for line in TITLE {
            assert!(line.iter().sum::<i32>() <= 80);
        }
    }
}
