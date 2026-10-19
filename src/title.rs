//! The main menu (`MainMenu` in armcom.py): a sunset sky, clouds and hills
//! scrolling past the title, the menu, and the poppy. Ported line for
//! line, with Continue for the saved game; the tombstone from the bones
//! file is not ported yet.

use crate::color::{self, Color};
use crate::console::{Align, COLCTRL_1, Console};
use crate::rng::Rng;
use std::time::{Duration, Instant};

pub const SCREEN_WIDTH: i32 = 149;
pub const SCREEN_HEIGHT: i32 = 61;
const SCREEN_XM: i32 = SCREEN_WIDTH / 2;

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
    background: Console,
    title: Console,
    clouds: Console,
    ground: Console,
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
}

impl MainMenu {
    pub fn new(rng: &mut Rng) -> Self {
        // the sunset sky
        let mut background = Console::new(149, 30);
        let color1 = Color(255, 255, 114);
        let color2 = Color(193, 8, 3);
        for y in 0..30 {
            background.bg = color::lerp(color1, color2, y as f32 / 30.0);
            background.rect(0, y, 149, 1, false);
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

        let mut clouds = Console::new(SCREEN_WIDTH, 30);
        clouds.bg = KEY_COLOR;
        clouds.clear();
        clouds.key = Some(KEY_COLOR);
        clouds.bg = color::WHITE;

        let mut ground = Console::new(SCREEN_WIDTH, 30);
        ground.bg = KEY_COLOR;
        ground.clear();
        ground.bg = TITLE_GROUND_COLOR;
        ground.key = Some(KEY_COLOR);

        let mut con = Console::new(SCREEN_WIDTH, SCREEN_HEIGHT);
        con.set_color_control(COLCTRL_1, KEY_HIGHLIGHT_COLOR, color::BLACK);

        let mut menu = MainMenu {
            con,
            background,
            title,
            clouds,
            ground,
            cloud_altitude: 0,
            cloud_height: 0,
            cloud_length: 0,
            c_cloud_length: 0,
            ground_height: 0,
            ground_click: Instant::now(),
            cloud_click: Instant::now(),
            saved: crate::save::peek(),
            confirm_new: false,
        };
        for _ in 0..SCREEN_WIDTH {
            menu.update_cloud_layer(rng);
        }
        menu.ground_height = rng.int(3, 8);
        for x in 0..SCREEN_WIDTH {
            menu.ground.rect(x, 30 - menu.ground_height, 1, menu.ground_height, false);
            menu.ground_height = new_ground_height(rng, menu.ground_height);
        }
        menu.draw_menu();
        menu.draw_layers();
        menu
    }

    /// `UpdateCloudLayer`: scroll the clouds one column left and draw the
    /// new right-hand column.
    fn update_cloud_layer(&mut self, rng: &mut Rng) {
        let c = &mut self.clouds;
        c.key = Some(color::BLACK);
        c.blit_self(1, 0, SCREEN_WIDTH - 1, 30, 0, 0);
        c.key = Some(KEY_COLOR);
        c.bg = KEY_COLOR;
        c.rect(SCREEN_WIDTH - 1, 0, 1, 30, false);
        c.bg = color::WHITE;

        if self.c_cloud_length > 0 {
            let mut h = self.cloud_height;
            if self.c_cloud_length == self.cloud_length || self.c_cloud_length == 1 {
                h -= 2;
            }
            c.rect(SCREEN_WIDTH - 1, 30 - self.cloud_altitude - h / 2, 1, h, false);
            self.c_cloud_length -= 1;
        } else if rng.int(1, 14) == 1 {
            self.cloud_altitude = rng.int(10, 29);
            self.cloud_height = *rng.choice(&[3, 5, 7]);
            self.cloud_length = rng.int(10, 17);
            self.c_cloud_length = self.cloud_length;
        }
    }

    fn scroll_ground(&mut self, rng: &mut Rng) {
        let g = &mut self.ground;
        let gh = self.ground_height;
        g.key = Some(color::BLACK);
        g.blit_self(1, 0, SCREEN_WIDTH - 1, 30, 0, 0);
        g.key = Some(KEY_COLOR);
        g.bg = KEY_COLOR;
        g.rect(SCREEN_WIDTH - 1, 0, 1, 30 - gh, false);
        g.bg = TITLE_GROUND_COLOR;
        g.rect(SCREEN_WIDTH - 1, 30 - gh, 1, gh, false);
        self.ground_height = new_ground_height(rng, gh);
    }

    /// `DrawLayers`.
    fn draw_layers(&mut self) {
        Console::blit(&self.background, 0, 0, 149, 30, &mut self.con, 0, 2);
        Console::blit(&self.clouds, 0, 0, 149, 30, &mut self.con, 0, 2);
        Console::blit(&self.ground, 0, 0, 149, 30, &mut self.con, 0, 2);
        Console::blit(&self.title, 0, 0, 80, 25, &mut self.con, 34, 4);
    }

    fn draw_menu(&mut self) {
        let con = &mut self.con;
        con.bg = color::BLACK;
        con.clear();
        con.align = Align::Center;
        con.print(SCREEN_XM, 33, "The World War II Tank Commander Roguelike");

        if let Some(saved) = &self.saved {
            con.print(SCREEN_XM, 36, &format!("[{HL_ON}C{HL_OFF}]ontinue Campaign:"));
            match saved {
                Ok(info) => {
                    con.fg = color::LIGHT_GREY;
                    for (n, line) in [&info.campaign_name, &info.commander_name, &info.tank_name, &info.current_date].iter().enumerate() {
                        con.print(SCREEN_XM, 38 + n as i32, line);
                    }
                }
                Err(e) => {
                    con.fg = color::LIGHT_RED;
                    con.print(SCREEN_XM, 38, "The saved game cannot be loaded:");
                    con.print(SCREEN_XM, 39, e);
                }
            }
            con.fg = color::WHITE;
        }
        if self.confirm_new {
            con.fg = color::LIGHT_RED;
            con.print(SCREEN_XM, 48, "Starting a new campaign will erase the currently saved one in progress.");
            con.fg = color::WHITE;
            con.print(SCREEN_XM, 49, &format!("Are you sure? [{HL_ON}y{HL_OFF}] or [{HL_ON}N{HL_OFF}]"));
        }

        con.print(SCREEN_XM, 43, &format!("[{HL_ON}N{HL_OFF}]ew Campaign"));
        con.print(SCREEN_XM, 44, &format!("[{HL_ON}H{HL_OFF}]igh Scores"));
        con.print(SCREEN_XM, 45, &format!("[{HL_ON}V{HL_OFF}]iew Credits"));
        con.print(SCREEN_XM, 46, &format!("[{HL_ON}Q{HL_OFF}]uit"));

        con.fg = color::LIGHT_GREY;
        con.print_ex(SCREEN_XM, SCREEN_HEIGHT - 7, Align::Center, &format!("version {VERSION}{SUBVERSION}"));
        con.print_ex(SCREEN_XM, SCREEN_HEIGHT - 4, Align::Center, "Copyright 2015-2017 Gregory Adam Scott");
        con.print_ex(SCREEN_XM, SCREEN_HEIGHT - 3, Align::Center, "Free Software under the GNU General Public License");
        con.print_ex(SCREEN_XM, SCREEN_HEIGHT - 2, Align::Center, WEBSITE);
        con.print_ex(SCREEN_XM, SCREEN_HEIGHT - 1, Align::Center, GITHUB);

        // the poppy
        con.fg = color::RED;
        con.align = Align::Left;
        con.print(4, SCREEN_HEIGHT - 6, ",--.");
        con.print(3, SCREEN_HEIGHT - 5, ".\\  /.");
        con.print(2, SCREEN_HEIGHT - 4, "(  ()  )");
        con.print(3, SCREEN_HEIGHT - 3, "`/  \\'");
        con.print(4, SCREEN_HEIGHT - 2, "`--'");

        con.fg = color::WHITE;
        con.align = Align::Center;
        let text = if is_armistice_day() { "NEVER AGAIN" } else { "In Memoriam" };
        con.print(16, SCREEN_HEIGHT - 3, text);
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
                self.draw_menu();
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
