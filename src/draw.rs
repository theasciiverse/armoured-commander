//! Encounter screen drawing ("Console Drawing and Updating" in
//! armcom.py), laid out for 100x30: `PaintMapCon`, `UpdateMapOverlay`,
//! the units' `DrawMe`, `UpdateTankCon`, `DisplayCrewOrders`,
//! `UpdateMsgCon`, `UpdateDateCon`, `DisplayWeather`, `DisplayMenuBar`
//! and `RenderEncounter`.

use crate::color::{self, Color};
use crate::console::{Align, Console};
use crate::game::*;
use crate::glyph::named;
use crate::layout::*;
use crate::model::*;
use crate::ui::wrap;

/// The six neighbours of a hex and the cells of the edge shared with
/// each, relative to the hex centre (see `layout::HEX_ROWS`).
const HEX_EDGES: [((i32, i32), &[(i32, i32)]); 6] = [
    ((0, -1), &[(-2, -2), (-1, -2), (0, -2), (1, -2), (2, -2), (3, -2)]),
    ((0, 1), &[(-2, 2), (-1, 2), (0, 2), (1, 2), (2, 2), (3, 2)]),
    ((1, -1), &[(4, -1), (5, 0)]),
    ((1, 0), &[(5, 1), (4, 2)]),
    ((-1, 0), &[(-3, -1), (-4, 0)]),
    ((-1, 1), &[(-4, 1), (-3, 2)]),
];

/// Where the spotting-sector highlight runs from and to, for each sector
/// (`UpdateMapOverlay`), scaled from the original's to the new hexes.
const SPOT_LINES: [((i32, i32), (i32, i32)); 6] = [
    ((2, 1), (27, 7)),
    ((0, 2), (0, 14)),
    ((-2, 1), (-27, 7)),
    ((-2, -1), (-27, -7)),
    ((0, -2), (0, -14)),
    ((2, -1), (27, -7)),
];

/// `DrawHex`, for the terminal hexes: the hex's own cells take its
/// background (its top edge row belongs to the hex above, as underscores
/// sit at the bottom of their cells), the edge glyphs take the default
/// foreground.
pub fn draw_hex(con: &mut Console, x: i32, y: i32) {
    for (dx, dy, text) in HEX_ROWS {
        for (i, ch) in text.bytes().enumerate() {
            let cx = x + dx + i as i32;
            let cy = y + dy;
            if dy == -2 {
                // top edge: draw the line, leave the cell's colour alone
                con.put_char(cx, cy, ch);
            } else {
                con.put_char_bg(cx, cy, ch);
            }
        }
    }
}

/// The phase instructions, shortened to fit 40 columns (the original's
/// `SPOT_SECTOR_INFO`, `ORDERS_PHASE_INFO` and the rest run to 60
/// characters). Every key the original lists is here.
pub fn phase_hints(phase: &str) -> &'static [&'static str] {
    match phase {
        "Set Spot Sectors" => &["[W/S] crewman  [A/D] turn sector", "[Space] go on to spotting"],
        "Orders" => &["[W/S] crewman  [A/D] order  [O] list", "[H] hatch  [R] reload type  [T] RR", "[Space] go on to crew actions"],
        "Issue Order" => &["[W/S] choose  [O] set order", "[Backspace] keep current order"],
        "Pivot Tank" => &["[A/D] pivot  [Enter] done"],
        "Rotate Turret" => &["[A/D] rotate turret  [Enter] done"],
        "Fire Main Gun" => &["[Enter] fire  [Tab] next target", "[A/D] turret  [F] area/direct fire", "[R] reload type  [T] RR  [Space] done"],
        "Fire MGs" => &["[Enter] fire  [Tab] next target", "[M] next MG  [A/D] turret (co-ax)", "[Space] done"],
        _ => &[],
    }
}

/// Short forms for the 40-column crew list; the full ones are on the
/// crew screen.
fn short_position(p: &str) -> &str {
    match p {
        "Commander" => "Commander",
        "Asst. Driver" => "Asst. Driver",
        other => other,
    }
}

impl Game {
    /// `DisplayWeather`: a 10x4 block of sky, precipitation, fog, ground.
    pub fn display_weather(con: &mut Console, w: &Weather, x: i32, y: i32) {
        if w.clouds == "Clear" {
            con.bg = CLEAR_SKY_COLOR;
            con.fg = color::WHITE;
        } else {
            con.bg = OVERCAST_COLOR;
            con.fg = color::DARK_GREY;
        }
        con.rect(x, y, 10, 3, false);
        con.print_ex_bg(x + 5, y, Align::Center, &w.clouds);
        if w.precip == "Rain" || w.precip == "Snow" {
            con.fg = color::WHITE;
            con.print_ex_bg(x + 5, y + 1, Align::Center, &w.precip);
        }
        if w.fog {
            con.bg = color::LIGHT_GREY;
            con.fg = color::WHITE;
            con.rect(x, y + 2, 10, 1, false);
            con.print_ex_bg(x + 5, y + 2, Align::Center, "Fog");
        }
        con.fg = color::BLACK;
        con.bg = match w.ground.as_str() {
            "Dry" => OPEN_GROUND_COLOR,
            "Mud" => MUD_COLOR,
            _ => color::WHITE,
        };
        con.rect(x, y + 3, 10, 1, false);
        con.print_ex_bg(x + 5, y + 3, Align::Center, &w.ground);
    }

    /// `PaintMapCon`: the hexes in their range shades, fog, smoke, and the
    /// sector boundaries.
    pub fn paint_map_con(&mut self) {
        let fog = self.campaign.weather.fog || self.campaign.weather.precip == "Snow";
        let b = self.battle.as_ref().expect("in a battle");
        let m = &mut self.map_con;
        m.bg = color::BLACK;
        m.clear();

        m.fg = HEX_EDGE_COLOR;
        for h in &b.maphexes {
            m.bg = match h.rng {
                2 => Color(80, 120, 80),
                1 => Color(90, 130, 90),
                _ => Color(100, 140, 100),
            };
            if h.hx == 0 && h.hy == 0 {
                m.fg = color::BLACK;
            }
            draw_hex(m, h.x, h.y);
        }
        m.bg = color::BLACK;

        if fog {
            for h in b.maphexes.iter().filter(|h| h.rng > 0) {
                m.bg = Color(180, 180, 180);
                m.fg = HEX_EDGE_COLOR;
                draw_hex(m, h.x, h.y);
                m.bg = color::BLACK;
            }
        }

        for h in &b.maphexes {
            if fog && h.rng > 0 {
                continue;
            }
            if h.smoke_factors > 0 {
                let c = (120 + h.smoke_factors * 20).min(220) as u8;
                m.bg = Color(c, c, c);
                m.fg = if h.hx == 0 && h.hy == 0 { color::BLACK } else { HEX_EDGE_COLOR };
                draw_hex(m, h.x, h.y);
                m.bg = color::BLACK;
            }
        }

        // zone boundaries: black where an edge separates two sectors
        for h in &b.maphexes {
            if h.sector < 0 {
                continue;
            }
            for ((dx, dy), cells) in HEX_EDGES {
                let Some(n) = b.maphexes.iter().find(|n| n.hx == h.hx + dx && n.hy == h.hy + dy) else { continue };
                if n.sector < 0 || n.sector == h.sector {
                    continue;
                }
                for (cx, cy) in cells {
                    if let Some(mut cell) = m.cell(h.x + cx, h.y + cy) {
                        cell.fg = color::BLACK;
                        m.set_cell(h.x + cx, h.y + cy, cell);
                    }
                }
            }
        }
    }

    /// The player's tank on the overlay (`PlayerTank.DrawMe`).
    fn draw_player_tank(&mut self) {
        fn turret_char(facing: i32) -> (i32, i32, u8) {
            match facing {
                0 => (MAP_X0 + 1, MAP_Y0 + 1, b'\\'),
                1 => (MAP_X0, MAP_Y0 + 1, b'|'),
                2 => (MAP_X0 - 1, MAP_Y0 + 1, b'/'),
                3 => (MAP_X0 - 1, MAP_Y0 - 1, b'\\'),
                4 => (MAP_X0, MAP_Y0 - 1, b'|'),
                _ => (MAP_X0 + 1, MAP_Y0 - 1, b'/'),
            }
        }
        let phase = self.bt().phase.clone();
        let t = &self.tank;
        let o = &mut self.overlay_con;
        let mc = &self.map_con;
        o.fg = color::WHITE;
        o.put_char_bg(MAP_X0, MAP_Y0, named::RADIO_UNSET);
        o.set_char_bg(MAP_X0, MAP_Y0, PLAYER_COLOR);

        let (x, y, ch) = turret_char(t.turret_facing);
        o.put_char_ex(x, y, ch, color::WHITE, mc.char_bg(x, y));

        if ["Fire Main Gun", "Fire MGs", "Rotate Turret"].contains(&phase.as_str()) && t.old_t_facing != t.turret_facing {
            let (x, y, ch) = turret_char(t.old_t_facing);
            o.put_char_ex(x, y, ch, color::DARK_GREY, mc.char_bg(x, y));
        } else if phase == "Pivot Tank" {
            let (x, y, ch, white_fg) = match t.new_facing {
                0 => (MAP_X0 + 3, MAP_Y0 + 1, 224, false),
                1 => (MAP_X0, MAP_Y0 + 2, 31, true),
                2 => (MAP_X0 - 3, MAP_Y0 + 1, 225, false),
                3 => (MAP_X0 - 3, MAP_Y0 - 1, 224, true),
                4 => (MAP_X0, MAP_Y0 - 2, 30, true),
                _ => (MAP_X0 + 3, MAP_Y0 - 1, 225, true),
            };
            let bg = mc.char_bg(x, y);
            if white_fg {
                o.put_char_ex(x, y, ch, color::WHITE, bg);
            } else {
                o.put_char_ex(x, y, ch, bg, color::WHITE);
            }
        }
    }

    /// `EnemyUnit.DrawMe`
    fn draw_unit(&mut self, i: usize) {
        let phase = self.bt().phase.clone();
        let target = self.bt().target;
        let u = self.u(i).clone();
        if !u.alive {
            return;
        }
        let o = &mut self.overlay_con;
        if u.hidden {
            o.bg = color::DARK_GREY;
            o.fg = color::BLACK;
        } else if !u.spotted {
            o.bg = ENEMY_COLOR;
            o.fg = color::DARKER_GREY;
        } else if u.is_class(&["TANK", "SPG", "APC", "AC"]) && !u.identified {
            o.bg = ENEMY_COLOR;
            o.fg = color::LIGHTER_GREY;
        } else {
            o.bg = ENEMY_COLOR;
            o.fg = color::WHITE;
        }
        if target == Some(i) && (phase == "Fire Main Gun" || phase == "Fire MGs") {
            o.bg = SELECTED_COLOR;
        }
        let ch = match u.unit_class.as_str() {
            "TANK" => named::RADIO_UNSET,
            "SPG" => b'#',
            "APC" => named::BULLET_INV,
            "AC" => named::RADIO_SET,
            "AT_GUN" => b'X',
            "MG" => b'x',
            "LW" => named::BLOCK1,
            _ => named::BULLET_SQUARE,
        };
        o.put_char_bg(u.x, u.y, ch);

        if u.hidden || !u.spotted || u.animating {
            return;
        }
        let ch = match u.terrain.as_str() {
            "Hull Down" => named::ARROW2_N,
            "Woods" => named::SPADE,
            "Building" => named::DVLINE,
            _ => return,
        };
        let bc = self.map_con.char_bg(u.x, u.y);
        let fc = Color(
            (bc.0 as u32 * color::LIGHT_GREY.0 as u32 / 255) as u8,
            (bc.1 as u32 * color::LIGHT_GREY.1 as u32 / 255) as u8,
            (bc.2 as u32 * color::LIGHT_GREY.2 as u32 / 255) as u8,
        );
        for dx in [-1, 1] {
            self.overlay_con.put_char_ex(u.x + dx, u.y, ch, fc, bc);
        }
    }

    /// `UpdateMapOverlay`: units, highlights, weather and the phase.
    pub fn update_map_overlay(&mut self, skip_los: bool) {
        let o = &mut self.overlay_con;
        o.fg = color::BLACK;
        o.bg = KEY_COLOR;
        o.clear();

        self.draw_player_tank();
        for i in 0..self.bt().enemy_units.len() {
            self.draw_unit(i);
        }

        let b = self.battle.as_ref().expect("in a battle");
        let o = &mut self.overlay_con;
        o.fg = color::WHITE;
        o.bg = color::BLACK;

        if b.phase == "Set Spot Sectors" {
            if let Some(c) = b.selected_crew {
                let s = self.tank.crew[c].spot_sector;
                let ((sx, sy), (ex, ey)) = SPOT_LINES[s.rem_euclid(6) as usize];
                let ex = (MAP_X0 + ex).clamp(0, MAP_CON_WIDTH - 1);
                let ey = (MAP_Y0 + ey).clamp(0, MAP_CON_HEIGHT - 1);
                for (x, y) in get_line(MAP_X0 + sx, MAP_Y0 + sy, ex, ey) {
                    let col = self.map_con.char_bg(x, y);
                    o.put_char_ex(x, y, 250, color::WHITE, col);
                }
            }
        }

        if (b.phase == "Fire Main Gun" || b.phase == "Fire MGs") && !skip_los {
            if let Some(t) = b.target {
                let u = &b.enemy_units[t];
                let line = get_line(MAP_X0, MAP_Y0, u.x, u.y);
                if line.len() > 3 {
                    for &(x, y) in &line[2..line.len() - 1] {
                        let col = self.map_con.char_bg(x, y);
                        o.put_char_ex(x, y, 250, color::WHITE, col);
                    }
                }
            }
        }

        Game::display_weather(o, &self.campaign.weather, MAP_CON_WIDTH - 10, 0);

        // round and phase: top left corner, clear of the hexes
        o.fg = color::WHITE;
        o.bg = color::BLACK;
        o.print_ex_bg(1, 0, Align::Left, &format!("Round {}", b.rounds_passed));
        if b.phase != "None" {
            o.print_ex_bg(1, 1, Align::Left, &format!("{} Phase", b.phase));
        }

        if b.phase == "Fire MGs" {
            let t = &self.tank;
            for (n, (can, label)) in [(t.coax_mg_can_fire, "Co-ax MG"), (t.bow_mg_can_fire, "Bow MG"), (t.aa_mg_can_fire, "AA MG")]
                .into_iter()
                .enumerate()
            {
                o.fg = if !can {
                    GREYED_COLOR
                } else if t.active_mg == n as i32 {
                    SELECTED_COLOR
                } else {
                    color::WHITE
                };
                o.print_ex_bg(1, 2 + n as i32, Align::Left, label);
            }
            o.fg = color::WHITE;
        }

        if self.campaign.weather.fog || self.campaign.weather.precip == "Snow" {
            o.print_ex_bg(0, MAP_CON_HEIGHT - 1, Align::Left, "Fog/Snow: close range only");
        }
    }

    /// `UpdateMapInfoCon`: what is under the pointer. Here it is a small
    /// box drawn beside the unit (see `render_encounter`) rather than a
    /// panel under the map; returns its lines.
    pub fn map_info_lines(&self, mx: i32, my: i32) -> Vec<(String, Color)> {
        let mut lines = Vec::new();
        if mx < MAP_CON_X || my < MAP_CON_Y || my >= MAP_CON_Y + MAP_CON_HEIGHT {
            return lines;
        }
        let (mx, my) = (mx - MAP_CON_X, my - MAP_CON_Y);
        if mx == MAP_X0 && my == MAP_Y0 {
            lines.push((format!("M4 Sherman \"{}\"", self.tank.name), color::WHITE));
            return lines;
        }
        let b = self.bt();
        let Some(u) = b.enemy_units.iter().find(|u| u.alive && u.x == mx && u.y == my) else {
            return lines;
        };
        let i = b.enemy_units.iter().position(|v| std::ptr::eq(v, u)).unwrap_or(0);
        lines.push((self.unit_desc(i, false), color::WHITE));
        if u.pinned {
            lines.push(("Pinned".into(), color::LIGHT_RED));
        } else if u.stunned {
            lines.push(("Stunned".into(), color::LIGHT_RED));
        }
        let h = &b.maphexes[u.map_hex];
        let rng = match h.rng {
            2 => "Long",
            1 => "Medium",
            _ => "Short",
        };
        let sector = match h.sector {
            0 => "Rear Right",
            1 => "Rear",
            2 => "Rear Left",
            3 => "Front Left",
            4 => "Front",
            _ => "Front Right",
        };
        // the original builds the range text and then overwrites it with
        // the sector; both are shown here
        lines.push((format!("{rng} Range, {sector} Sector"), color::WHITE));
        if u.hidden || !u.spotted {
            return lines;
        }
        let mut text = if u.unit_class == "AT_GUN" {
            "Emplaced"
        } else if u.moving {
            "Moving"
        } else if u.immobile {
            "Immobile"
        } else {
            "Stationary"
        }
        .to_string();
        text.push_str(&format!(" in {}", u.terrain));
        lines.push((text, color::WHITE));
        if !u.facing.is_empty() {
            lines.push((format!("{} Facing", u.facing), color::WHITE));
        }
        let mut text = String::new();
        if u.acquired > 0 {
            text.push_str(&format!("Acquired Target: {}", u.acquired));
        }
        if u.acquired > 0 && u.acquired_player > 0 {
            text.push_str(" ; ");
        }
        if u.acquired_player > 0 {
            text.push_str(&format!("Acquired Player: {}", u.acquired_player));
        }
        if u.unit_class == "AC" && u.spotting_player {
            text.push_str("Spotting Player");
        }
        if !text.is_empty() {
            lines.push((text, color::WHITE));
        }
        if !(u.is_class(&["TANK", "SPG", "AT_GUN"]) && !u.identified) {
            lines.push(("Right-click for more info".into(), color::LIGHT_GREY));
        }
        lines
    }

    /// `UpdateMsgCon`: phase instructions, then the newest messages that
    /// fit below them.
    pub fn update_msg_con(&mut self) {
        let Some(b) = &self.battle else { return };
        let m = &mut self.msg_con;
        m.clear();
        let info = phase_hints(&b.phase);
        let mut lines: Vec<String> = Vec::new();
        let mut y = 0;
        if b.phase == "Fire Main Gun" {
            m.fg = HIGHLIGHT_COLOR;
            m.print(0, y, if b.area_fire { "Area Fire Mode" } else { "Direct Fire Mode" });
            y += 1;
        }
        m.fg = color::WHITE;
        for line in info {
            lines.push(line.replace('[', &format!("[{HL_ON}")).replace(']', &format!("{HL_OFF}]")));
        }
        for line in &lines {
            m.print(0, y, line);
            y += 1;
        }
        if !lines.is_empty() {
            m.fg = color::DARK_GREY;
            m.hline(0, y, MSG_CON_WIDTH);
            y += 1;
        }
        let room = (m.h - y).max(0) as usize;
        let start = b.messages.len().saturating_sub(room);
        for (line, col) in &b.messages[start..] {
            m.fg = *col;
            m.print(0, y, line);
            y += 1;
        }
        m.fg = color::WHITE;
    }

    /// `UpdateTankCon`: the tank and crew, in 40 columns.
    pub fn update_tank_con(&mut self) {
        self.tank_con.clear();
        if let Some(b) = &self.battle {
            if b.phase == "Issue Order" {
                self.display_crew_orders();
                return;
            }
        }
        let in_battle = self.battle.is_some();
        let t = &self.tank;
        let c = &mut self.tank_con;
        let w = TANK_CON_WIDTH;

        c.fg = HIGHLIGHT_COLOR;
        c.print(1, 0, &t.name);
        c.fg = color::WHITE;
        let mut text = t.stats.str("vehicle_type").to_string();
        if t.stats.has("nickname") {
            text.push_str(&format!(" \"{}\"", t.stats.str("nickname")));
        }
        if t.stats.has("HVSS") {
            text.push_str(" (HVSS)");
        }
        c.print(1, 1, &text);

        if in_battle {
            c.fg = color::LIGHT_GREY;
            let mut text = String::new();
            if t.lead_tank {
                text.push_str("Lead Tank, ");
            }
            text.push_str(if t.moving { "Moving" } else { "Stopped" });
            if t.hull_down {
                c.fg = color::LIGHT_GREEN;
                text.push_str(", Hull Down");
            }
            if t.bogged_down {
                c.fg = color::RED;
                text.push_str(", Bogged Down");
            }
            if t.immobilized {
                c.fg = color::RED;
                text.push_str(", Immobilized");
            }
            c.print(1, 2, &text);
            c.fg = color::WHITE;
        }

        // ammo: types across, general stores and ready rack below
        c.bg = ROW_COLOR;
        c.rect(1, 3, w - 2, 1, false);
        c.bg = color::BLACK;
        c.fg = color::LIGHT_GREY;
        c.print(1, 3, "Ammo");
        c.print(1, 4, "General");
        c.print(1, 5, "Ready Rack");
        let mut x = 11;
        let (mut total_g, mut total_rr) = (0, 0);
        for a in AMMO_TYPES {
            if !t.general_ammo.has(a) {
                continue;
            }
            x += a.len() as i32 + 2;
            c.fg = color::LIGHT_GREY;
            c.print_ex(x, 3, Align::Right, a);
            c.fg = color::WHITE;
            c.print_ex(x, 4, Align::Right, &t.general_ammo.get(a).to_string());
            c.print_ex(x, 5, Align::Right, &t.rr_ammo.get(a).to_string());
            total_g += t.general_ammo.get(a);
            total_rr += t.rr_ammo.get(a);
            if self.campaign.resupply {
                let extra = match a {
                    "HCBI" => self.campaign.hcbi,
                    "HVAP" => self.campaign.hvap,
                    "APDS" => self.campaign.apds,
                    _ => 0,
                };
                if extra > 0 {
                    c.fg = color::LIGHT_GREY;
                    c.print_ex(x, 2, Align::Right, &format!("({extra})"));
                }
            }
        }
        c.fg = color::LIGHT_GREY;
        c.print_ex(w - 2, 3, Align::Right, "Max");
        c.fg = color::WHITE;
        c.print_ex(w - 2, 4, Align::Right, &format!("{total_g}/{}", t.stats.int("main_gun_rounds")));
        c.print_ex(w - 2, 5, Align::Right, &format!("{total_rr}/{}", t.stats.int("rr_size")));
        if self.campaign.resupply && !in_battle {
            c.fg = color::LIGHT_GREY;
            c.print(1, 2, "Resupplying");
        }

        // main gun, load, reload, smoke
        c.bg = ROW_COLOR;
        c.rect(1, 6, w - 2, 1, false);
        c.bg = color::BLACK;
        let mut x = 1;
        let mut field = |c: &mut Console, label: &str, value: &str, col: Color| {
            c.fg = color::LIGHT_GREY;
            c.print(x, 6, label);
            x += label.len() as i32;
            c.fg = col;
            c.print(x, 6, value);
            x += value.len() as i32 + 1;
        };
        field(c, "", &format!("{}mm", t.stats.str("main_gun").replace('L', "")), color::WHITE);
        let load_col = if t.ammo_load == "None" { color::RED } else { color::WHITE };
        field(c, "Load ", &t.ammo_load, load_col);
        field(c, "Next ", &t.ammo_reload, color::WHITE);
        field(c, "", if t.use_rr { "RR" } else { "Gen" }, color::WHITE);
        let smoke = if t.stats.has("smoke_mortar") {
            format!("{}/{}", t.smoke_grenades, t.smoke_bombs)
        } else {
            t.smoke_grenades.to_string()
        };
        field(c, "Smoke ", &smoke, color::WHITE);

        // damage
        let mut text = String::new();
        if t.swiss_cheese {
            text.push_str("Suffered Penetrating Hit! ");
        }
        for d in &t.damage_list {
            if !text.is_empty() {
                text.push_str("; ");
            }
            text.push_str(d);
        }
        if text.is_empty() {
            c.fg = color::LIGHT_GREY;
            c.print(1, 7, "No damage");
        } else {
            c.fg = color::RED;
            let lines = wrap(&text, (w - 2) as usize, "");
            c.print(1, 7, &lines[0]);
        }

        // crew, two rows each
        let selected = match &self.battle {
            Some(b) if ["Set Spot Sectors", "Orders"].contains(&b.phase.as_str()) => b.selected_crew,
            Some(_) => None,
            None => self.campaign.selected_crew,
        };
        let mut y = 8;
        for (n, cm) in t.crew.iter().enumerate() {
            c.bg = if n % 2 == 1 { ROW_COLOR } else { ROW_COLOR2 };
            c.rect(0, y, w, 2, false);
            c.bg = color::BLACK;
            let mut fc = color::WHITE;
            if cm.no_actions() {
                fc = GREYED_COLOR;
            }
            if selected == Some(n) {
                fc = HIGHLIGHT_COLOR;
                c.fg = HIGHLIGHT_COLOR;
                c.put_char(0, y, 16);
            }
            c.fg = fc;
            c.print(1, y, short_position(&cm.position));
            let rank = &self.campaign.ranks.get(cm.rank_level).map(|r| r.0.clone()).unwrap_or_default();
            let wound = cm.v_serious_wound || cm.serious_wound || cm.light_wound || !cm.alive;
            c.fg = if wound && fc == color::WHITE { color::LIGHT_RED } else { fc };
            let name = format!("{rank} {}", cm.name);
            c.print(14, y, &name.chars().take((w - 15) as usize).collect::<String>());

            c.fg = fc;
            // hatch and spot on the right, the order in what is left
            let mut right = String::new();
            if cm.hatch != "None" {
                right.push_str(&cm.hatch);
                right.push_str(" \u{fa} ");
            }
            right.push_str(&cm.spot);
            let room = (w - 3 - right.len() as i32).max(0) as usize;
            if in_battle {
                let order: String = cm.order.chars().take(room).collect();
                c.print(2, y + 1, &order);
            } else if cm.no_actions() || wound {
                c.fg = color::RED;
                c.print(2, y + 1, &Game::wound_text(cm));
                c.fg = fc;
            }
            c.fg = if fc == color::WHITE { color::LIGHT_GREY } else { fc };
            c.print_ex(w - 1, y + 1, Align::Right, &right);
            y += 2;
        }
        c.fg = color::WHITE;
    }

    /// A crewman's wounds and state, as the tank console shows them.
    pub fn wound_text(c: &Crewman) -> String {
        if !c.alive {
            return "Dead".into();
        }
        let mut text = if c.v_serious_wound {
            "Very Serious Wound"
        } else if c.serious_wound {
            "Serious Wound"
        } else if c.light_wound {
            "Light Wound"
        } else {
            ""
        }
        .to_string();
        if c.unconscious {
            if !text.is_empty() {
                text.push_str(", ");
            }
            text.push_str("Unconscious");
        } else if c.stunned {
            if !text.is_empty() {
                text.push_str(", ");
            }
            text.push_str("Stunned");
        }
        text
    }

    /// `DisplayCrewOrders`: the order list for the selected crewman.
    pub fn display_crew_orders(&mut self) {
        let b = self.battle.as_mut().expect("in a battle");
        let ci = b.selected_crew.expect("a crewman is selected");
        let cm = &self.tank.crew[ci];
        if b.selected_order.is_none() {
            b.selected_order = Some(cm.orders_list.iter().position(|o| o.name == cm.order).unwrap_or(0));
        }
        let sel = b.selected_order.unwrap_or(0);
        let c = &mut self.tank_con;
        let w = TANK_CON_WIDTH;
        c.fg = color::WHITE;
        c.print(1, 0, "Set Crew Order for:");
        c.bg = ROW_COLOR;
        c.rect(1, 1, w - 2, 2, false);
        c.bg = color::BLACK;
        c.print(1, 1, &format!("{}  {}", cm.position, cm.name));
        c.fg = color::LIGHT_GREY;
        let mut info = format!("Hatch {}  Spot {}", cm.hatch, cm.spot);
        if !cm.nickname.is_empty() {
            info = format!("\"{}\"  {info}", cm.nickname);
        }
        c.print(1, 2, &info);
        c.fg = color::WHITE;

        // the list scrolls if it is longer than the room for it
        let list_rows = 8;
        let first = sel.saturating_sub(list_rows - 1);
        let mut y = 4;
        for (n, order) in cm.orders_list.iter().enumerate().skip(first).take(list_rows) {
            if n == sel {
                c.bg = SELECTED_COLOR;
            }
            c.print_ex_bg(1, y, Align::Left, &order.name);
            c.bg = color::BLACK;
            y += 1;
        }
        let mut dy = y + 1;
        if let Some(order) = cm.orders_list.get(sel) {
            for line in wrap(&order.desc, (w - 2) as usize, " ") {
                if dy >= c.h - 1 {
                    break;
                }
                c.print(1, dy, &line);
                dy += 1;
            }
            let text = format!("{} next turn", if order.spot { "May spot" } else { "May not spot" });
            c.fg = color::LIGHT_GREY;
            c.print(1, c.h - 1, &text);
            c.fg = color::WHITE;
        }
    }

    /// `UpdateDateCon`: date and time, mission and terrain (or the day's
    /// expected resistance outside an encounter).
    pub fn update_date_con(&mut self) {
        let c = &mut self.date_con;
        c.clear();
        c.print(0, 0, &self.campaign.get_date(None));
        c.print_ex(DATE_CON_WIDTH - 1, 0, Align::Right, &format!("{}:{:02}", self.campaign.hour, self.campaign.minute));
        let mut text = self.campaign.scen_type.clone();
        if self.battle.as_ref().is_some_and(|b| b.counterattack) {
            text = "Counterattack".into();
        }
        c.print(0, 1, &text);
        let right = if self.battle.is_none() {
            format!("Day Resistance: {}", self.campaign.scen_res)
        } else if self.campaign.day_map.is_some() {
            self.campaign.terrain_desc(self.campaign.player_node()).to_string()
        } else {
            String::new()
        };
        c.print_ex(DATE_CON_WIDTH - 1, 1, Align::Right, &right);
    }

    /// `DisplayMenuBar`, shortened to fit 100 columns.
    pub fn display_menu_bar(&mut self) {
        let c = &mut self.con;
        c.fg = color::LIGHT_GREY;
        c.align = Align::Left;
        let mut x = 1;
        if self.campaign.day_in_progress {
            c.print(x, 0, &format!("{}:Menu  ", hl("ESC")));
            x += 11;
        }
        let bar = format!(
            "{}:Help  {}:Tank  {}:Crew  {}:Settings  {}:Stats  {}:Sound",
            hl("F1"),
            hl("F2"),
            hl("F3"),
            hl("F4"),
            hl("F5"),
            hl("F7")
        );
        c.print(x, 0, &bar);
        c.fg = color::WHITE;
    }

    /// `RenderEncounter`: compose the encounter screen and show it.
    pub fn render_encounter(&mut self, no_flush: bool) {
        if self.battle.is_none() {
            return;
        }
        self.con.clear();
        self.display_menu_bar();
        Console::blit(&self.date_con, 0, 0, DATE_CON_WIDTH, DATE_CON_HEIGHT, &mut self.con, DATE_CON_X, DATE_CON_Y);
        Console::blit(&self.tank_con, 0, 0, TANK_CON_WIDTH, TANK_CON_HEIGHT, &mut self.con, TANK_CON_X, TANK_CON_Y);
        self.update_msg_con();
        Console::blit(&self.msg_con, 0, 0, MSG_CON_WIDTH, MSG_CON_HEIGHT, &mut self.con, MSG_CON_X, MSG_CON_Y);
        Console::blit(&self.map_con, 0, 0, MAP_CON_WIDTH, MAP_CON_HEIGHT, &mut self.con, MAP_CON_X, MAP_CON_Y);
        Console::blit(&self.overlay_con, 0, 0, MAP_CON_WIDTH, MAP_CON_HEIGHT, &mut self.con, MAP_CON_X, MAP_CON_Y);

        self.con.fg = color::DARK_GREY;
        self.con.vline(LEFT_W, 1, SCREEN_HEIGHT - 1);
        self.con.fg = color::WHITE;

        // what is under the pointer, beside it
        let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
        let lines = self.map_info_lines(mx, my);
        if !lines.is_empty() {
            let w = lines.iter().map(|(l, _)| l.chars().count() as i32).max().unwrap_or(0) + 2;
            let h = lines.len() as i32 + 2;
            let mut x = mx + 2;
            if x + w > SCREEN_WIDTH {
                x = mx - w - 1;
            }
            let y = (my - h / 2).clamp(1, SCREEN_HEIGHT - h);
            let c = &mut self.con;
            c.fg = color::LIGHT_GREY;
            c.bg = color::BLACK;
            c.frame(x, y, w, h, true, true);
            for (n, (line, col)) in lines.iter().enumerate() {
                c.fg = *col;
                c.print(x + 1, y + 1 + n as i32, line);
            }
            c.fg = color::WHITE;
        }

        self.blit_con();
        if !no_flush {
            self.flush();
        }
    }
}
