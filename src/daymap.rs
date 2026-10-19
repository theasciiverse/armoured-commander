//! "Campaign Day Map" in armcom.py, `GetPath`, `GenerateCampaignMap` and
//! `PaintCampaignMap`, and the campaign day screen: `UpdateCOverlay`,
//! `UpdateCActionCon`, `UpdateCInfoCon` and `RenderCampaign`, laid out for
//! 100x30 (see `layout`).
//!
//! The original builds the roads inside `PaintCampaignMap`, which runs
//! twice for every new map (once at the end of `GenerateCampaignMap`, once
//! after it), so most maps got a second network of roads laid over the
//! first. Here the roads are built once, when the map is generated, and
//! painting only paints.

use crate::color::{self, Color};
use crate::console::{Align, Console};
use crate::game::*;
use crate::glyph::named;
use crate::layout::*;
use crate::model::*;
use crate::rng::Rng;

/// `MIN_DIST`: the least distance between area centres.
const MIN_DIST: i32 = 7;
/// `NUM_NODES`: how many areas to try for.
const NUM_NODES: usize = 55;

/// The day map's colours by season: field, woods ground, coniferous tree,
/// deciduous tree, village ground, marsh ground.
type Scheme = [(i32, i32, i32); 6];
const LATE_SUMMER_TO_MID_AUTUMN: Scheme = [(140, 110, 16), (16, 60, 16), (80, 110, 80), (80, 110, 80), (60, 90, 60), (60, 30, 10)];
const MID_TO_LATE_AUTUMN: Scheme = [(120, 90, 16), (16, 60, 16), (80, 110, 80), (200, 80, 20), (60, 90, 60), (60, 30, 10)];
const EDGE_OF_WINTER: Scheme = [(110, 90, 45), (70, 45, 15), (80, 110, 80), (80, 50, 30), (80, 70, 45), (60, 90, 45)];
const WINTER_OR_GROUND_SNOW: Scheme =
    [(240, 240, 240), (240, 240, 240), (80, 110, 80), (80, 50, 30), (240, 240, 240), (240, 240, 240)];
const SPRING: Scheme = [(110, 170, 110), (36, 80, 36), (80, 110, 80), (230, 135, 210), (60, 90, 60), (60, 30, 10)];
const SUMMER: Scheme = [(90, 150, 90), (16, 60, 16), (80, 110, 80), (80, 110, 80), (60, 90, 60), (60, 30, 10)];

/// The day's actions and the area actions' instructions, shortened to fit
/// the 39-column panel (the original's `CHECK_AREA` and `MOVE_AREA` run to
/// 55 characters).
const CHECK_AREA: [&str; 5] = [
    "Check an Adjacent Area",
    "",
    "[\u{1}Tab\u{8}] next area, Shift to reverse",
    "[\u{1}Enter\u{8}] check it for resistance",
    "[\u{1}Backspace\u{8}] cancel",
];
const MOVE_AREA: [&str; 5] = [
    "Move into an Adjacent Area",
    "",
    "[\u{1}Tab\u{8}] next area, Shift to reverse",
    "[\u{1}Enter\u{8}] move into the area",
    "[\u{1}Backspace\u{8}] cancel the move",
];

/// `GetPath`: A* over the area links, the start left out of the path;
/// empty if there is none. Woods cost 100 to enter, other areas 1. With
/// `enemy_blocks`, enemy-held areas cannot be entered. The original takes
/// the first of its open set with the lowest score, in the set's own
/// order; this takes the first one added.
pub fn get_path(map: &CampaignDayMap, node1: usize, node2: usize, enemy_blocks: bool) -> Vec<usize> {
    let n = map.nodes.len();
    let dist = |a: usize, b: usize| {
        let (p, q) = (&map.nodes[a], &map.nodes[b]);
        get_distance(p.x, p.y, q.x, q.y)
    };
    let mut g = vec![0; n];
    let mut h = vec![0; n];
    let mut f = vec![0; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];
    let mut open: Vec<usize> = Vec::new();
    let mut in_open = vec![false; n];
    let mut closed = vec![false; n];

    h[node1] = dist(node1, node2);
    f[node1] = g[node1] + h[node1];
    open.push(node1);
    in_open[node1] = true;

    while !open.is_empty() {
        let (pos, current) = open.iter().copied().enumerate().min_by_key(|&(_, i)| f[i]).expect("open list");
        if current == node2 {
            let mut path = vec![current];
            let mut node = current;
            while let Some(p) = parent[node] {
                path.push(p);
                node = p;
            }
            path.reverse();
            path.remove(0);
            return path;
        }
        open.remove(pos);
        in_open[current] = false;
        closed[current] = true;

        for &node in &map.nodes[current].links {
            if closed[node] || map.blocked_nodes.contains(&node) {
                continue;
            }
            if enemy_blocks && !map.nodes[node].friendly_control {
                continue;
            }
            let cost = if map.nodes[node].node_type == "D" { 100 } else { 1 };
            let gg = g[current] + cost;
            if !in_open[node] {
                g[node] = gg;
                h[node] = dist(node, node2);
                f[node] = g[node] + h[node];
                parent[node] = Some(current);
                open.push(node);
                in_open[node] = true;
            } else if gg < g[node] {
                parent[node] = Some(current);
                g[node] = gg;
                f[node] = g[node] + h[node];
            }
        }
    }
    Vec::new()
}

impl Game {
    /// `GetPath` on today's map.
    pub fn get_path(&self, node1: usize, node2: usize, enemy_blocks: bool) -> Vec<usize> {
        get_path(self.campaign.day_map(), node1, node2, enemy_blocks)
    }

    /// `random.sample(nodes, len(nodes))`: the area indices in a random
    /// order.
    fn shuffled_nodes(&mut self) -> Vec<usize> {
        let mut list: Vec<usize> = (0..self.campaign.day_map().nodes.len()).collect();
        self.shuffle(&mut list);
        list
    }

    /// `GenerateCampaignMap`: a new map for the day, its roads and paint;
    /// false if the start and exit cannot be joined, to try again.
    pub fn generate_campaign_map(&mut self) -> bool {
        let (w, h) = (c_map_width_for_screen(), C_MAP_CON_HEIGHT);
        let mut map = CampaignDayMap { w, h, ..Default::default() };
        self.campaign.selected_node = None;

        for _ in 0..300 {
            let x = self.rng.int(3, w - 4);
            let y = self.rng.int(3, h - 4);
            if map.nodes.iter().any(|n| get_distance(x, y, n.x, n.y) <= MIN_DIST) {
                continue;
            }
            map.nodes.push(MapNode::new(x, y));
            if map.nodes.len() >= NUM_NODES {
                break;
            }
        }

        // every cell belongs to the nearest area centre, the first if tied
        for y in 0..h {
            for x in 0..w {
                let mut nearest = 0;
                let mut nearest_dist = i32::MAX;
                for (i, n) in map.nodes.iter().enumerate() {
                    let d = get_distance(x, y, n.x, n.y);
                    if d < nearest_dist {
                        nearest = i;
                        nearest_dist = d;
                    }
                }
                map.char_locations.insert((x, y), nearest);
            }
        }

        // edge cells, and links between areas that touch
        for y in 0..h {
            for x in 0..w {
                let p = map.char_locations[&(x, y)];
                if x - 1 < 0 || x + 1 >= w || y - 1 < 0 || y + 1 >= h {
                    map.nodes[p].edges.insert((x, y));
                    continue;
                }
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    let p2 = map.char_locations[&(x + dx, y + dy)];
                    if p2 != p {
                        map.nodes[p].edges.insert((x, y));
                        if !map.nodes[p].links.contains(&p2) {
                            map.nodes[p].links.push(p2);
                            map.nodes[p2].links.push(p);
                        }
                    }
                }
            }
        }

        // the day's terrain. The forest table stops at 11, so on a 12 the
        // original left an area with no terrain at all, which crashed when
        // it was captured (`AwardCaptureVP`); the last entry covers it here.
        let terrain = self.campaign.today().and_then(|d| d.terrain.clone());
        let chances: &[(i32, &str)] = match terrain.as_deref() {
            Some("bocage") => &[(4, "C"), (5, "A"), (8, "F"), (10, "B"), (11, "D"), (12, "E")],
            Some("forest") => &[(3, "C"), (4, "A"), (8, "D"), (10, "B"), (11, "E")],
            _ => &[(4, "C"), (5, "A"), (8, "B"), (9, "D"), (12, "E")],
        };
        for i in 0..map.nodes.len() {
            let mut links = map.nodes[i].links.clone();
            links.sort_by_key(|&l| (map.nodes[l].y, map.nodes[l].x));
            map.nodes[i].links = links;

            let (_, _, roll) = self.roll_2d6();
            let node_type = chances.iter().find(|(s, _)| roll <= *s).unwrap_or(&chances[chances.len() - 1]).1;
            let node = &mut map.nodes[i];
            node.node_type = node_type.to_string();
            if node_type == "C" {
                // village buildings reach to just short of the nearest edge
                let closest = node.edges.iter().map(|&(x, y)| get_distance(x, y, node.x, node.y)).min().unwrap_or(100).min(100);
                node.village_radius = closest - 1;
                if node.village_radius == 0 {
                    node.village_radius = 1;
                }
            } else if node_type == "E" {
                map.blocked_nodes.insert(i);
            }
        }
        self.campaign.day_map = Some(map);

        // no two villages side by side
        for i in self.shuffled_nodes() {
            let map = self.campaign.day_map_mut();
            if map.nodes[i].node_type == "C" {
                let links = map.nodes[i].links.clone();
                for l in links {
                    if map.nodes[l].node_type == "C" {
                        map.nodes[i].node_type = "A".into();
                    }
                }
            }
        }

        // marshland is impassable: cut its links
        let map = self.campaign.day_map_mut();
        for &b in &map.blocked_nodes.clone() {
            for l in std::mem::take(&mut map.nodes[b].links) {
                map.nodes[l].links.retain(|&x| x != b);
            }
        }

        for x in 0..w {
            let top = map.char_locations[&(x, 0)];
            map.nodes[top].top_edge = true;
            let bottom = map.char_locations[&(x, h - 1)];
            map.nodes[bottom].bottom_edge = true;
        }
        for y in 0..h {
            let left = map.char_locations[&(0, y)];
            map.nodes[left].left_edge = true;
            let right = map.char_locations[&(w - 1, y)];
            map.nodes[right].right_edge = true;
        }

        // start on the bottom edge (top in a counterattack), exit on the
        // opposite one
        let counterattack = self.campaign.scen_type == "Counterattack";
        let mut start_node = None;
        for i in self.shuffled_nodes() {
            let map = self.campaign.day_map_mut();
            if map.blocked_nodes.contains(&i) {
                continue;
            }
            let n = &mut map.nodes[i];
            if if counterattack { !n.top_edge } else { !n.bottom_edge } {
                continue;
            }
            n.start = true;
            n.friendly_control = true;
            map.player_node = i;
            start_node = Some(i);
            break;
        }
        let mut exit_node = None;
        for i in self.shuffled_nodes() {
            let map = self.campaign.day_map_mut();
            let n = &map.nodes[i];
            if if counterattack { !n.bottom_edge } else { !n.top_edge } {
                continue;
            }
            if !map.blocked_nodes.contains(&i) {
                map.nodes[i].exit = true;
                exit_node = Some(i);
                break;
            }
        }
        let (Some(start), Some(exit)) = (start_node, exit_node) else { return false };
        if self.get_path(start, exit, false).is_empty() {
            return false;
        }

        // the roads, then each area's resistance. The original built the
        // roads after the resistance rolls, so their modifiers never
        // applied; fixed (David, 2026-10-19).
        let seed = self.rng.int(0, 2_147_483_647) as u64;
        self.campaign.day_map_mut().seed = seed;
        self.generate_roads();
        for i in 0..self.campaign.day_map().nodes.len() {
            if self.campaign.day_map().blocked_nodes.contains(&i) {
                continue;
            }
            let mut roll = self.roll_1d10();
            let n = &self.campaign.day_map().nodes[i];
            roll += match n.node_type.as_str() {
                "A" => 1,
                "B" => -1,
                "C" => 2,
                "D" => -2,
                _ => 0,
            };
            if !n.stone_road_links.is_empty() {
                roll += 2;
            }
            if !n.dirt_road_links.is_empty() {
                roll += 1;
            }
            let res = match self.campaign.scen_res.as_str() {
                "Light" => {
                    if roll <= 7 { "Light" } else { "Medium" }
                }
                "Medium" => {
                    if roll <= 5 {
                        "Light"
                    } else if roll <= 9 {
                        "Medium"
                    } else {
                        "Heavy"
                    }
                }
                _ => {
                    if roll <= 4 {
                        "Light"
                    } else if roll <= 8 {
                        "Medium"
                    } else {
                        "Heavy"
                    }
                }
            };
            let n = &mut self.campaign.day_map_mut().nodes[i];
            n.resistance = Some(res.to_string());
            if counterattack {
                n.friendly_control = true;
            }
        }

        self.paint_campaign_map();
        true
    }

    /// The roads of `PaintCampaignMap`: most maps have an improved road
    /// from the bottom edge to the top, some a crossroad from left to
    /// right, and villages off them are joined by dirt roads.
    fn generate_roads(&mut self) {
        let mut rng = Rng::new(self.campaign.day_map().seed ^ 0x005E_ED0F_A0AD);

        if rng.int(1, 10) <= 8 {
            let (mut node1, mut node2) = (None, None);
            for i in self.shuffled_nodes() {
                let map = self.campaign.day_map();
                let n = &map.nodes[i];
                let ok = n.node_type != "D" && !map.blocked_nodes.contains(&i);
                if n.bottom_edge && ok {
                    node1 = Some(i);
                } else if n.top_edge && ok {
                    node2 = Some(i);
                }
                if node1.is_some() && node2.is_some() {
                    break;
                }
            }
            if let (Some(a), Some(b)) = (node1, node2) {
                self.generate_road(a, b, false);
            }
        }

        if rng.int(1, 10) <= 2 {
            let (mut node1, mut node2) = (None, None);
            for i in self.shuffled_nodes() {
                let n = &self.campaign.day_map().nodes[i];
                if n.top_edge || n.bottom_edge {
                    continue;
                }
                if n.left_edge && n.node_type != "D" {
                    node1 = Some(i);
                } else if n.right_edge && n.node_type != "D" {
                    node2 = Some(i);
                }
                if node1.is_some() && node2.is_some() {
                    break;
                }
            }
            if let (Some(a), Some(b)) = (node1, node2) {
                self.generate_road(a, b, false);
            }
        }

        // villages off the roads: a dirt road to the nearest road, or with
        // no roads at all, to the nearest village or dirt road
        let count = self.campaign.day_map().nodes.len();
        for n1 in 0..count {
            let map = self.campaign.day_map();
            let a = &map.nodes[n1];
            if a.node_type != "C" || !a.stone_road_links.is_empty() || !a.dirt_road_links.is_empty() {
                continue;
            }
            let nearest = |pick: &dyn Fn(&MapNode) -> bool| {
                let mut closest: Option<usize> = None;
                for n2 in 0..count {
                    if n2 == n1 || !pick(&map.nodes[n2]) {
                        continue;
                    }
                    let b = &map.nodes[n2];
                    match closest {
                        None => closest = Some(n2),
                        Some(c) => {
                            let cn = &map.nodes[c];
                            if get_distance(a.x, a.y, b.x, b.y) < get_distance(a.x, a.y, cn.x, cn.y) {
                                closest = Some(n2);
                            }
                        }
                    }
                }
                closest
            };
            let to_road = nearest(&|n: &MapNode| !n.stone_road_links.is_empty() || !n.dirt_road_links.is_empty());
            let to = to_road.or_else(|| nearest(&|n: &MapNode| n.node_type == "C" || !n.dirt_road_links.is_empty()));
            if let Some(n2) = to {
                self.generate_road(n1, n2, true);
            }
        }
    }

    /// `GenerateRoad`: link the areas along the path from one to the other.
    fn generate_road(&mut self, node1: usize, node2: usize, dirt: bool) {
        let path = self.get_path(node1, node2, false);
        if path.is_empty() {
            return;
        }
        let map = self.campaign.day_map_mut();
        let mut last = node1;
        for node in path {
            if dirt {
                map.nodes[last].dirt_road_links.push(node);
                map.nodes[node].dirt_road_links.push(last);
            } else {
                map.nodes[last].stone_road_links.push(node);
                map.nodes[node].stone_road_links.push(last);
            }
            last = node;
        }
        if !dirt {
            map.nodes[node1].road_end = true;
            map.nodes[node2].road_end = true;
        }
    }

    /// `PaintCampaignMap`: the ground, buildings, trees, roads and bocage,
    /// from the map's own seed so that it paints the same every time.
    pub fn paint_campaign_map(&mut self) {
        let mut rng = Rng::new(self.campaign.day_map().seed);
        let month = self.campaign.current_date[1];
        let (scheme, name) = if ["Snow", "Deep Snow"].contains(&self.campaign.weather.ground.as_str()) {
            (WINTER_OR_GROUND_SNOW, "WINTER_OR_GROUND_SNOW")
        } else if [11, 12, 1, 2, 3].contains(&month) {
            (EDGE_OF_WINTER, "EDGE_OF_WINTER")
        } else if [4, 5].contains(&month) {
            (SPRING, "SPRING")
        } else if [6, 7].contains(&month) {
            (SUMMER, "SUMMER")
        } else if [8, 9].contains(&month) {
            (LATE_SUMMER_TO_MID_AUTUMN, "LATE_SUMMER_TO_MID_AUTUMN")
        } else {
            (MID_TO_LATE_AUTUMN, "MID_TO_LATE_AUTUMN")
        };
        self.campaign.color_scheme = Some(name.to_string());

        let map = self.campaign.day_map.as_mut().expect("day map");
        let (w, h) = (map.w, map.h);
        if (self.c_map_con.w, self.c_map_con.h) != (w, h) {
            self.c_map_con = create_console(w, h, color::BLACK, color::BLACK, Align::Left);
            self.c_overlay_con = create_console(w, h, KEY_COLOR, color::WHITE, Align::Left);
            self.c_overlay_con.key = Some(KEY_COLOR);
        }
        let c = &mut self.c_map_con;
        c.bg = color::BLACK;
        c.clear();

        for y in 0..h {
            for x in 0..w {
                let p = &map.nodes[map.char_locations[&(x, y)]];
                let mut fc = color::BLACK;
                let mut ch = 0u8;
                let mut bc;
                match p.node_type.as_str() {
                    "A" | "B" | "F" => {
                        bc = color::shift(scheme[0], rng.int(-3, 7));
                        if p.node_type == "A" && rng.int(1, 50) == 1 {
                            bc = color::GREY;
                            fc = color::LIGHT_GREY;
                            ch = 179;
                        }
                    }
                    "D" => {
                        bc = color::shift(scheme[1], rng.int(-5, 10));
                        if rng.int(1, 10) > 6 {
                            let c_mod = rng.int(-20, 20);
                            if rng.int(1, 8) == 1 {
                                ch = named::ARROW2_N;
                                fc = color::shift(scheme[2], c_mod);
                            } else {
                                ch = named::SPADE;
                                fc = color::shift(scheme[3], c_mod);
                            }
                        }
                    }
                    "C" => {
                        bc = color::shift(scheme[4], rng.int(-5, 10));
                        let dist = get_distance(x, y, p.x, p.y);
                        if dist <= p.village_radius {
                            let chance = (100.0 * (dist as f64 / p.village_radius as f64)) as i32;
                            if rng.int(1, 120) >= chance {
                                bc = color::shift((80, 50, 30), rng.int(-5, 10));
                                if rng.int(1, 3) == 3 {
                                    fc = color::LIGHT_GREY;
                                    ch = 254;
                                }
                            }
                        }
                    }
                    _ => {
                        bc = if rng.int(1, 3) <= 2 {
                            color::shift((10, 30, 60), rng.int(-5, 10))
                        } else {
                            color::shift(scheme[4], rng.int(-5, 10))
                        };
                        if rng.int(1, 8) == 1 {
                            if rng.int(1, 5) == 1 {
                                fc = color::shift(scheme[3], rng.int(-20, 20));
                                ch = if rng.int(1, 8) == 1 { named::ARROW2_N } else { named::SPADE };
                            } else {
                                let c_mod = rng.int(-5, 10);
                                fc = Color(16, (60 + c_mod) as u8, 16);
                                ch = 19;
                            }
                        }
                    }
                }
                if p.edges.contains(&(x, y)) {
                    bc = color::mul(bc, color::LIGHTER_GREY);
                    fc = color::mul(fc, color::LIGHTER_GREY);
                }
                c.put_char_ex(x, y, ch, fc, bc);
            }
        }

        let draw_road = |c: &mut Console, rng: &mut Rng, line: Vec<(i32, i32)>, dirt: bool| {
            for (x, y) in line {
                let c_mod = rng.int(-5, 10);
                let col = if dirt { color::shift((80, 50, 30), c_mod) } else { color::shift((60, 60, 60), c_mod) };
                c.put_char_ex(x, y, 219, col, col);
            }
        };
        for dirt in [true, false] {
            let mut skip = Vec::new();
            for n1 in 0..map.nodes.len() {
                let links = if dirt { &map.nodes[n1].dirt_road_links } else { &map.nodes[n1].stone_road_links };
                if links.is_empty() {
                    continue;
                }
                skip.push(n1);
                for &n2 in links {
                    if skip.contains(&n2) {
                        continue;
                    }
                    let (a, b) = (&map.nodes[n1], &map.nodes[n2]);
                    draw_road(c, &mut rng, get_line(a.x, a.y, b.x, b.y), dirt);
                }
            }
        }

        // improved roads run on to the map edge; of two road ends side by
        // side only one does, which makes a fork
        for n in &mut map.nodes {
            n.extended = false;
        }
        for i in 0..map.nodes.len() {
            let n = &map.nodes[i];
            if !n.road_end || n.links.iter().any(|&l| map.nodes[l].extended) {
                continue;
            }
            let line = if n.top_edge {
                get_line(n.x, n.y, n.x, 0)
            } else if n.bottom_edge {
                get_line(n.x, n.y, n.x, h - 1)
            } else if n.left_edge {
                get_line(n.x, n.y, 0, n.y)
            } else if n.right_edge {
                get_line(n.x, n.y, w - 1, n.y)
            } else {
                continue;
            };
            draw_road(c, &mut rng, line, false);
            map.nodes[i].extended = true;
        }

        // bocage: hedges round the area and four boxes inside it
        let draw_bocage = |c: &mut Console, rng: &mut Rng, x: i32, y: i32| {
            let col = color::shift((20, 60, 20), rng.int(-5, 10));
            c.put_char_ex(x, y, 219, col, col);
        };
        for i in 0..map.nodes.len() {
            if map.nodes[i].node_type != "F" {
                continue;
            }
            let mut locations = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    if map.char_locations[&(x, y)] == i {
                        locations.push((x, y));
                    }
                }
            }
            for &(x, y) in &map.nodes[i].edges {
                draw_bocage(c, &mut rng, x, y);
            }
            let inside = |x: i32, y: i32| map.char_locations.get(&(x, y)) == Some(&i);
            for _ in 0..4 {
                let (x, y) = locations[rng.int(0, locations.len() as i32 - 1) as usize];
                let w = rng.int(3, 9);
                let h = rng.int(3, 9);
                for x1 in x - w..=x + w {
                    for y1 in [y - h, y + h] {
                        if inside(x1, y1) && c.char_at(x1, y1) == 0 {
                            draw_bocage(c, &mut rng, x1, y1);
                        }
                    }
                }
                for y1 in y - h + 1..y + h {
                    for x1 in [x - w, x + w] {
                        if inside(x1, y1) && c.char_at(x1, y1) == 0 {
                            draw_bocage(c, &mut rng, x1, y1);
                        }
                    }
                }
            }
        }
    }

    /// `UpdateCOverlay`
    pub fn update_c_overlay(&mut self, highlight_node: Option<usize>) {
        self.update_c_overlay_anim(highlight_node, None);
    }

    /// `UpdateCOverlay`: the front line, area labels, the player and the
    /// selected area; `anim` draws the player moving instead.
    pub fn update_c_overlay_anim(&mut self, highlight_node: Option<usize>, anim: Option<(i32, i32)>) {
        let Some(map) = &self.campaign.day_map else { return };
        let camp = &self.campaign;
        let o = &mut self.c_overlay_con;
        o.bg = KEY_COLOR;
        o.clear();
        o.bg = color::BLACK;

        // the front line, on the enemy side of each friendly area's edge
        o.fg = FRONTLINE_COLOR;
        for (i, node) in map.nodes.iter().enumerate() {
            if !node.friendly_control || map.blocked_nodes.contains(&i) {
                continue;
            }
            for &(x, y) in &node.edges {
                for (x2, y2) in [(x, y - 1), (x - 1, y), (x + 1, y), (x, y + 1)] {
                    let Some(&n2) = map.char_locations.get(&(x2, y2)) else { continue };
                    if n2 != i && !map.nodes[n2].friendly_control {
                        o.put_char_bg(x2, y2, 178);
                    }
                }
            }
        }

        o.fg = if camp.color_scheme.as_deref() == Some("WINTER_OR_GROUND_SNOW") { color::BLUE } else { color::WHITE };

        let selected = camp.selected_node.filter(|_| camp.input_mode != "None");
        if let Some(s) = selected {
            if ["Move Into Adjacent Area", "Call in Strike"].contains(&camp.input_mode.as_str()) {
                let (p, n) = (&map.nodes[map.player_node], &map.nodes[s]);
                for (x, y) in get_line(p.x, p.y, n.x, n.y) {
                    o.put_char_bg(x, y, 250);
                }
            }
        }

        for (i, node) in map.nodes.iter().enumerate() {
            if node.start {
                o.print_ex_bg(node.x, node.y - 1, Align::Center, "Start");
            } else if node.exit {
                o.print_ex_bg(node.x, node.y - 1, Align::Center, "Exit");
            }
            if map.player_node == i && anim.is_none() {
                o.put_char_bg(node.x, node.y, b'@');
                for &(x, y) in &node.edges {
                    o.put_char_bg(x, y, named::BULLET);
                }
            } else {
                o.put_char_bg(node.x, node.y, named::BULLET);
            }
            if node.friendly_control {
                o.print_ex_bg(node.x, node.y + 1, Align::Center, &camp.player_nation);
            } else if node.res_known {
                if let Some(r) = &node.resistance {
                    o.print_ex_bg(node.x, node.y + 1, Align::Center, r);
                }
            }
            if highlight_node == Some(i) {
                let col = o.fg;
                o.fg = SELECTED_COLOR;
                for &(x, y) in &node.edges {
                    o.put_char_bg(x, y, named::BULLET);
                }
                o.fg = col;
            }
            if !node.friendly_control && (node.arty_strike || node.air_strike) {
                o.print_ex_bg(node.x, node.y + 2, Align::Center, "Area hit by");
                let text = if node.arty_strike { "Artillery" } else { "Air Strike" };
                o.print_ex_bg(node.x, node.y + 3, Align::Center, text);
            }
            if let Some(q) = &node.quest_type {
                o.print_ex_bg(node.x, node.y - 2, Align::Center, q);
            }
        }

        if let Some(s) = selected {
            o.fg = SELECTED_COLOR;
            for &(x, y) in &map.nodes[s].edges {
                o.put_char_bg(x, y, 219);
            }
        }

        if let Some((x, y)) = anim {
            o.fg = color::WHITE;
            o.put_char_bg(x, y, b'@');
        }
    }

    /// `UpdateCActionCon`: the actions and their time, or the instructions
    /// for the action under way; time to sunset at the foot.
    pub fn update_c_action_con(&mut self) {
        let camp = &self.campaign;
        let c = &mut self.c_action_con;
        c.clear();
        let w = C_ACTION_CON_W;
        if camp.input_mode != "None" {
            let lines: Vec<String> = match camp.input_mode.as_str() {
                "Check Adjacent Area" => CHECK_AREA.iter().map(|s| s.to_string()).collect(),
                "Move Into Adjacent Area" => MOVE_AREA.iter().map(|s| s.to_string()).collect(),
                _ => {
                    let mut l = vec!["Call in Strike on Adjacent Area".to_string(), String::new()];
                    l.push(format!("[{}] next area, Shift to reverse", hl("Tab")));
                    l.push(format!("Call in [{}]rtillery Strike", hl("A")));
                    let w = &camp.weather;
                    if w.clouds != "Overcast" && !w.fog && w.precip != "Snow" {
                        l.push(format!("Call in Ai[{}] Strike", hl("r")));
                    }
                    l.push(format!("[{}] cancel", hl("Backspace")));
                    l
                }
            };
            for (dy, line) in lines.iter().enumerate() {
                c.print(0, dy as i32, line);
            }
        } else {
            c.fg = color::LIGHT_GREY;
            c.print(0, 0, "Action");
            c.print_ex(w - 1, 0, Align::Right, "Mins.");
            c.fg = color::WHITE;
            for (n, (text, time)) in camp.action_list.iter().enumerate() {
                let y = 1 + n as i32;
                c.print(0, y, text);
                let t = time.map_or("N/A".to_string(), |t| t.to_string());
                c.print_ex(w - 1, y, Align::Right, &t);
            }
        }

        c.print(0, C_ACTION_CON_H - 2, &format!("[{}]: Scroll Map", hl("W/S/Up/Down")));
        let (sh, sm) = camp.get_sunset();
        let (h, m) = get_time_until(camp.hour, camp.minute, sh, sm);
        if h < 0 {
            return;
        }
        c.print(0, C_ACTION_CON_H - 1, &format!("Time until sunset: {h}:{m:02}"));
    }

    /// The day map's view: its left edge on the screen, and how many
    /// columns and rows of the map it shows. A map narrower than the
    /// window is centred in it.
    pub fn c_map_view(&self) -> (i32, i32, i32) {
        let map = self.campaign.day_map();
        let (w, h) = (map.w.min(c_map_window_w()), map.h.min(c_map_window_h()));
        (C_MAP_CON_X + (c_map_window_w() - w) / 2, w, h)
    }

    /// The area under the pointer on the map, if it is over the map.
    fn c_map_area_at(&self, mx: i32, my: i32) -> Option<usize> {
        self.campaign.day_map.as_ref()?;
        let (left, w, h) = self.c_map_view();
        if mx < left || mx >= left + w || my < C_MAP_CON_Y || my >= C_MAP_CON_Y + h {
            return None;
        }
        let c = &self.campaign;
        c.day_map().char_locations.get(&(mx - left + c.c_map_x, my - C_MAP_CON_Y + c.c_map_y)).copied()
    }

    /// `UpdateCInfoCon`: strike chances and the weather, and what is known
    /// of the area under the pointer. Without a pointer over the map, the
    /// selected area, or the player's own, is described instead (the
    /// original needed the mouse for this).
    pub fn update_c_info_con(&mut self, mx: i32, my: i32) {
        let node = self.c_map_area_at(mx, my).or_else(|| {
            let camp = &self.campaign;
            camp.selected_node.filter(|_| camp.input_mode != "None").or(camp.day_map.as_ref().map(|m| m.player_node))
        });
        let camp = &self.campaign;
        let c = &mut self.c_info_con;
        c.bg = color::BLACK;
        c.clear();
        Game::display_weather(c, &camp.weather, C_INFO_CON_X - 5, 0);
        c.bg = color::BLACK;
        c.fg = color::WHITE;

        c.print(0, 0, "Artillery");
        c.print(0, 1, "Chance");
        c.print(2, 2, &format!("<={}", camp.arty_chance));
        let w = C_INFO_CON_W;
        c.print_ex(w - 1, 0, Align::Right, "Air Strike");
        c.print_ex(w - 1, 1, Align::Right, "Chance");
        let air = &camp.weather;
        let text = if air.clouds == "Overcast" || air.fog || air.precip == "Snow" {
            "N/A".to_string()
        } else {
            format!("<={}", camp.air_chance)
        };
        c.print_ex(w - 3, 2, Align::Right, &text);

        let (Some(i), Some(map)) = (node, &camp.day_map) else {
            c.print_ex(C_INFO_CON_X, 5, Align::Center, "Mouseover an area for info");
            return;
        };
        let n = &map.nodes[i];
        let text = match n.node_type.as_str() {
            "A" => "Farm Buildings and Fields",
            "B" => "Fields",
            "C" => "Village",
            "D" => "Woods",
            "E" => "Marshland",
            "F" => "Bocage",
            _ => "",
        };
        c.print_ex(C_INFO_CON_X, 5, Align::Center, text);
        let text = if n.friendly_control {
            "Friendly Control".to_string()
        } else if n.res_known {
            format!("{} Enemy Resistance Expected", n.resistance.as_deref().unwrap_or(""))
        } else {
            "Unknown Enemy Resistance Level".to_string()
        };
        c.print_ex(C_INFO_CON_X, 7, Align::Center, &text);
        if camp.quest_active {
            if let Some(q) = &n.quest_type {
                c.print_ex(C_INFO_CON_X, 9, Align::Center, &format!("Active Quest: {q}"));
                c.print_ex(C_INFO_CON_X, 10, Align::Center, &format!("VP Bonus: {}", n.quest_vp_bonus.unwrap_or(0)));
                if let Some((h, m)) = n.quest_time_limit {
                    c.print_ex(C_INFO_CON_X, 11, Align::Center, &format!("Expires: {h}:{m:02}"));
                }
            }
        }
    }

    /// Where an area's centre is on the screen.
    pub fn node_screen_pos(&self, i: usize) -> (i32, i32) {
        let n = &self.campaign.day_map().nodes[i];
        let (left, _, _) = self.c_map_view();
        (n.x + left - self.campaign.c_map_x, n.y + C_MAP_CON_Y - self.campaign.c_map_y)
    }

    /// `RenderCampaign`: compose the campaign day screen and show it.
    pub fn render_campaign(&mut self, no_flush: bool) {
        if self.campaign.day_map.is_none() {
            return;
        }
        // the info panel follows the selected or current area as well as
        // the pointer, so it is brought up to date on every frame
        let (mx, my) = (self.scr.mouse.cx, self.scr.mouse.cy);
        self.update_c_info_con(mx, my);
        self.con.bg = color::BLACK;
        self.con.clear();
        self.display_menu_bar();
        Console::blit(&self.date_con, 0, 0, DATE_CON_WIDTH, DATE_CON_HEIGHT, &mut self.con, DATE_CON_X, DATE_CON_Y);
        let (x0, y0) = (self.campaign.c_map_x, self.campaign.c_map_y);
        let (left, w, h) = self.c_map_view();
        Console::blit(&self.c_map_con, x0, y0, w, h, &mut self.con, left, C_MAP_CON_Y);
        Console::blit_fg(&self.c_overlay_con, x0, y0, w, h, &mut self.con, left, C_MAP_CON_Y);
        Console::blit(&self.c_action_con, 0, 0, C_ACTION_CON_W, C_ACTION_CON_H, &mut self.con, C_ACTION_CON_X, C_ACTION_CON_Y);
        Console::blit(&self.c_info_con, 0, 0, C_INFO_CON_W, C_INFO_CON_H, &mut self.con, 1, C_INFO_CON_Y);

        let c = &mut self.con;
        c.fg = color::DARK_GREY;
        c.hline(1, C_ACTION_CON_Y - 1, LEFT_W - 1);
        c.hline(1, C_INFO_CON_Y - 1, LEFT_W - 1);
        c.vline(LEFT_W, 1, sh() - 1);
        c.fg = color::WHITE;

        self.blit_con();
        if !no_flush {
            self.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Screen;

    /// A game on a day of one of the campaigns, by the seed: both
    /// campaigns, and days all through their calendars (plain, bocage and
    /// forest terrain).
    fn game(seed: u64, scen_type: &str) -> Game {
        let mut g = Game::new(Screen::headless(), Rng::new(seed));
        let files = crate::defs::CAMPAIGN_FILES;
        let def = crate::defs::load_campaign(files[seed as usize % files.len()].1).unwrap();
        g.campaign.load_info(&def);
        g.campaign.set_today(seed as usize % g.campaign.days.len());
        g.campaign.scen_type = scen_type.into();
        g.campaign.scen_res = "Medium".into();
        g
    }

    #[test]
    fn day_maps_join_start_to_exit() {
        let mut hops = 0;
        let mut maps = 0;
        for seed in 1..=120u64 {
            for scen in ["Advance", "Counterattack"] {
                let mut g = game(seed, scen);
                let mut tries = 0;
                while !g.generate_campaign_map() {
                    tries += 1;
                    assert!(tries < 50, "seed {seed}: no good map");
                }
                let map = g.campaign.day_map();
                let start = map.nodes.iter().position(|n| n.start).unwrap();
                let exit = map.nodes.iter().position(|n| n.exit).unwrap();
                assert_eq!(map.player_node, start);
                assert!(!map.blocked_nodes.contains(&start) && !map.blocked_nodes.contains(&exit));
                let path = g.get_path(start, exit, false);
                assert!(!path.is_empty());
                assert_eq!(*path.last().unwrap(), exit);
                for (i, n) in map.nodes.iter().enumerate() {
                    for &l in &n.links {
                        assert!(map.nodes[l].links.contains(&i), "links go both ways");
                    }
                    assert!(!n.node_type.is_empty());
                }
                hops += path.len();
                maps += 1;
            }
        }
        let avg = hops as f64 / maps as f64;
        eprintln!("average path from start to exit: {avg:.2} areas");
        assert!((4.0..12.0).contains(&avg));
    }

    #[test]
    fn painting_is_repeatable() {
        let mut g = game(5, "Advance");
        while !g.generate_campaign_map() {}
        let first = g.c_map_con.clone();
        g.paint_campaign_map();
        let map = g.campaign.day_map();
        for y in 0..map.h {
            for x in 0..map.w {
                assert_eq!(first.cell(x, y), g.c_map_con.cell(x, y));
            }
        }
    }
}
