//! Screen layout: 100x30 at least (agreed 2026-10-18), and the whole
//! terminal when it is bigger. The original's constants of the same names
//! were for its 149x61 window; those that follow the terminal's size are
//! functions here (`sw()` for `SCREEN_WIDTH` and so on).
//!
//! Encounter screen: menu bar on row 0; a 40-column panel on the left
//! (date, tank, crew, phase instructions, messages); a divider; the hex
//! map on the right, 59x29.

/// The smallest screen the game lays out for.
pub const MIN_WIDTH: i32 = 100;
pub const MIN_HEIGHT: i32 = 30;

static SCREEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(((MIN_WIDTH as u32) << 16) | MIN_HEIGHT as u32);

/// Set the screen size from the terminal's, never below 100x30.
pub fn set_screen_size(w: i32, h: i32) {
    let (w, h) = (w.max(MIN_WIDTH), h.max(MIN_HEIGHT));
    SCREEN.store(((w as u32) << 16) | h as u32, std::sync::atomic::Ordering::Relaxed);
}

/// The screen's width: the terminal's, at least 100 (`SCREEN_WIDTH`).
pub fn sw() -> i32 {
    (SCREEN.load(std::sync::atomic::Ordering::Relaxed) >> 16) as i32
}

/// The screen's height: the terminal's, at least 30 (`SCREEN_HEIGHT`).
pub fn sh() -> i32 {
    (SCREEN.load(std::sync::atomic::Ordering::Relaxed) & 0xFFFF) as i32
}

/// `SCREEN_XM`
pub fn sxm() -> i32 {
    sw() / 2
}

/// `SCREEN_YM`
pub fn sym() -> i32 {
    sh() / 2
}

/// Left panel: the original's tank, message and date consoles share it.
pub const LEFT_W: i32 = 40;

pub const DATE_CON_WIDTH: i32 = LEFT_W - 1;
pub const DATE_CON_HEIGHT: i32 = 2;
pub const DATE_CON_X: i32 = 1;
pub const DATE_CON_Y: i32 = 1;

pub const TANK_CON_WIDTH: i32 = LEFT_W - 1;
pub const TANK_CON_X: i32 = 0;
pub const TANK_CON_Y: i32 = DATE_CON_Y + DATE_CON_HEIGHT;
/// Tank and crew rows; the rest of the panel below is instructions and
/// messages, which `msg_con` fills.
pub const TANK_CON_HEIGHT: i32 = 18;

pub const MSG_CON_WIDTH: i32 = LEFT_W - 1;
pub const MSG_CON_X: i32 = 1;
pub const MSG_CON_Y: i32 = TANK_CON_Y + TANK_CON_HEIGHT;

/// The message log takes the rest of the left panel, so it grows with the
/// terminal.
pub fn msg_con_height() -> i32 {
    sh() - MSG_CON_Y
}

pub const MAP_CON_WIDTH: i32 = 59;
pub const MAP_CON_HEIGHT: i32 = 29;
pub const MAP_X0: i32 = MAP_CON_WIDTH / 2;
pub const MAP_Y0: i32 = MAP_CON_HEIGHT / 2;

/// The hex map, centred in the space right of the left panel.
pub fn map_con_x() -> i32 {
    LEFT_W + 1 + (sw() - LEFT_W - 1 - MAP_CON_WIDTH) / 2
}

pub fn map_con_y() -> i32 {
    1 + (sh() - 1 - MAP_CON_HEIGHT) / 2
}

/// Window consoles (`menu_con`, `text_con`) are 100x30 (the original's
/// were 139x42 and 86x57 inside 149x61), centred on a bigger screen.
pub const MENU_CON_WIDTH: i32 = MIN_WIDTH;
pub const MENU_CON_HEIGHT: i32 = MIN_HEIGHT;
pub const MENU_CON_XM: i32 = MENU_CON_WIDTH / 2;
pub const MENU_CON_YM: i32 = MENU_CON_HEIGHT / 2;

pub fn menu_con_x() -> i32 {
    (sw() - MENU_CON_WIDTH) / 2
}

pub fn menu_con_y() -> i32 {
    (sh() - MENU_CON_HEIGHT) / 2
}

pub const TEXT_CON_WIDTH: i32 = 86;
pub const TEXT_CON_HEIGHT: i32 = MIN_HEIGHT - 2;
pub const TEXT_CON_XM: i32 = TEXT_CON_WIDTH / 2;

pub fn text_con_x() -> i32 {
    sxm() - TEXT_CON_XM
}

pub fn text_con_y() -> i32 {
    (sh() - TEXT_CON_HEIGHT) / 2
}

/// Campaign day screen: the map window on the right as in the original,
/// the date, action and info panels on the left, one above the other.
///
/// The day map is generated as wide as the window, from 59 up to the
/// original's 90 columns, and 82 tall where the original's was 90. The
/// height is what keeps a day as long: the same generator, run 300 times
/// at each size, crosses 7.1 areas from start to exit at 90x90 and 7.0 at
/// 59x82 (about 40 areas on the map instead of 55). The width only gives
/// more areas side by side.
pub const C_MAP_MIN_WIDTH: i32 = 59;
pub const C_MAP_MAX_WIDTH: i32 = 90;
pub const C_MAP_CON_HEIGHT: i32 = 82;
pub const C_MAP_CON_X: i32 = LEFT_W + 1;
pub const C_MAP_CON_Y: i32 = 1;
pub const C_ACTION_CON_W: i32 = LEFT_W - 1;
pub const C_ACTION_CON_H: i32 = 13;
pub const C_ACTION_CON_X: i32 = 1;
pub const C_ACTION_CON_Y: i32 = DATE_CON_Y + DATE_CON_HEIGHT + 1;
pub const C_INFO_CON_W: i32 = LEFT_W - 1;
pub const C_INFO_CON_Y: i32 = C_ACTION_CON_Y + C_ACTION_CON_H + 1;
pub const C_INFO_CON_H: i32 = 12;
pub const C_INFO_CON_X: i32 = C_INFO_CON_W / 2;

/// The day map window: the screen right of the left panel.
pub fn c_map_window_w() -> i32 {
    sw() - C_MAP_CON_X
}

pub fn c_map_window_h() -> i32 {
    sh() - C_MAP_CON_Y
}

/// How wide to generate a day map for the screen as it is.
pub fn c_map_width_for_screen() -> i32 {
    c_map_window_w().clamp(C_MAP_MIN_WIDTH, C_MAP_MAX_WIDTH)
}

/// Hex spacing on the map: 8 columns across, 4 rows down, each column of
/// hexes half a hex lower than the one to its left (the original: 9 and
/// 6). Cells are twice as tall as wide, so the hexes keep their shape.
pub const HEX_DX: i32 = 8;
pub const HEX_DY: i32 = 4;

/// `Hex2Screen`: centre of a hex on the map console.
pub fn hex2screen(hx: i32, hy: i32) -> (i32, i32) {
    (MAP_X0 + hx * HEX_DX, MAP_Y0 + hy * HEX_DY + hx * HEX_DY / 2)
}

/// The rows of one hex, as (dx, dy, text) relative to its centre: a
/// 10-wide, 5-tall hex whose top and bottom edges are underscores, so
/// neighbours share their edges.
pub const HEX_ROWS: [(i32, i32, &str); 5] = [
    (-2, -2, "______"),
    (-3, -1, "/      \\"),
    (-4, 0, "/        \\"),
    (-4, 1, "\\        /"),
    (-3, 2, "\\______/"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hex_fits_the_map() {
        for h in &crate::defs::defs().hexes {
            let (x, y) = hex2screen(h.0, h.1);
            for (dx, dy, text) in HEX_ROWS {
                assert!(x + dx >= 0 && x + dx + text.len() as i32 <= MAP_CON_WIDTH, "hex {h:?}");
                assert!(y + dy >= 0 && y + dy < MAP_CON_HEIGHT, "hex {h:?}");
            }
        }
    }

    #[test]
    fn neighbours_share_edges() {
        // the hex to the lower right starts its top edge on our middle row
        let (x, y) = hex2screen(0, 0);
        let (x2, y2) = hex2screen(1, 0);
        assert_eq!((x2 - x, y2 - y), (8, 2));
        // and the one below starts where our bottom edge is
        let (_, y3) = hex2screen(0, 1);
        assert_eq!(y3 - 2, y + 2);
    }

    #[test]
    fn panels_fit_the_screen() {
        assert!(msg_con_height() >= 6);
        assert_eq!(map_con_x() + MAP_CON_WIDTH, MIN_WIDTH);
        assert_eq!(map_con_y() + MAP_CON_HEIGHT, MIN_HEIGHT);
        assert_eq!(C_MAP_CON_X + C_MAP_MIN_WIDTH, MIN_WIDTH);
        assert_eq!(C_INFO_CON_Y + C_INFO_CON_H, MIN_HEIGHT);
    }
}
