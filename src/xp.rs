//! REXPaint `.xp` images: gzip around a little-endian grid of cells, stored
//! column by column. ArmCom loads only the first layer of each
//! (`LoadXP` and `xp_loader.load_layer_to_console`), transparent cells
//! included, so this does the same.

use crate::color::Color;
use crate::console::{Cell, Console};
use flate2::read::GzDecoder;
use std::io::Read;

/// Every `.xp` file in the original `data/`, built into the binary.
pub const FILES: &[(&str, &[u8])] = &[
    ("canadas_best_map.xp", include_bytes!("../data/canadas_best_map.xp")),
    ("pattons_best_map.xp", include_bytes!("../data/pattons_best_map.xp")),
    ("m4_a.xp", include_bytes!("../data/m4_a.xp")),
    ("m4_b.xp", include_bytes!("../data/m4_b.xp")),
    ("m4_c.xp", include_bytes!("../data/m4_c.xp")),
    ("m4_vc.xp", include_bytes!("../data/m4_vc.xp")),
    ("m4_a1_a.xp", include_bytes!("../data/m4_a1_a.xp")),
    ("m4_a1_b.xp", include_bytes!("../data/m4_a1_b.xp")),
    ("m4_a1_c.xp", include_bytes!("../data/m4_a1_c.xp")),
    ("m4_a1_76_g.xp", include_bytes!("../data/m4_a1_76_g.xp")),
    ("m4_a1_76_h.xp", include_bytes!("../data/m4_a1_76_h.xp")),
    ("m4_a3_a.xp", include_bytes!("../data/m4_a3_a.xp")),
    ("m4_a3_b.xp", include_bytes!("../data/m4_a3_b.xp")),
    ("m4_a3_c.xp", include_bytes!("../data/m4_a3_c.xp")),
    ("m4_a3_75_d.xp", include_bytes!("../data/m4_a3_75_d.xp")),
    ("m4_a3_75_e.xp", include_bytes!("../data/m4_a3_75_e.xp")),
    ("m4_a3_76_g.xp", include_bytes!("../data/m4_a3_76_g.xp")),
    ("m4_a3_76_h.xp", include_bytes!("../data/m4_a3_76_h.xp")),
    ("m4_a3_e2_f.xp", include_bytes!("../data/m4_a3_e2_f.xp")),
    ("m4_a3_e2_75_f.xp", include_bytes!("../data/m4_a3_e2_75_f.xp")),
    ("m4_a3_e2_76_f.xp", include_bytes!("../data/m4_a3_e2_76_f.xp")),
];

#[allow(dead_code)]
pub fn by_name(name: &str) -> Option<Console> {
    FILES.iter().find(|(n, _)| *n == name).and_then(|(_, data)| load(data).ok())
}

pub fn load(gz: &[u8]) -> Result<Console, String> {
    let mut raw = Vec::new();
    GzDecoder::new(gz).read_to_end(&mut raw).map_err(|e| format!("gzip: {e}"))?;
    let mut at = 0;
    let mut int = |raw: &[u8]| -> Result<i32, String> {
        let b = raw.get(at..at + 4).ok_or("truncated")?;
        at += 4;
        Ok(i32::from_le_bytes(b.try_into().unwrap()))
    };
    let _version = int(&raw)?;
    let layers = int(&raw)?;
    if layers < 1 {
        return Err("no layers".into());
    }
    let w = int(&raw)?;
    let h = int(&raw)?;
    let start = 16;
    let need = start + (w * h) as usize * 10;
    if raw.len() < need {
        return Err("truncated layer".into());
    }
    let mut con = Console::new(w, h);
    for i in 0..(w * h) as usize {
        let c = &raw[start + i * 10..start + i * 10 + 10];
        let code = u32::from_le_bytes(c[0..4].try_into().unwrap());
        let cell = Cell {
            ch: code.min(255) as u8,
            fg: Color(c[4], c[5], c[6]),
            bg: Color(c[7], c[8], c[9]),
        };
        // Column-major: cell i is at x = i / h, y = i % h.
        con.set_cell(i as i32 / h, i as i32 % h, cell);
    }
    Ok(con)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_file_loads() {
        for (name, data) in FILES {
            let con = load(data).unwrap_or_else(|e| panic!("{name}: {e}"));
            assert!(con.w > 0 && con.h > 0, "{name}");
        }
    }

    #[test]
    fn sizes_match_the_originals() {
        assert_eq!(by_name("pattons_best_map.xp").map(|c| (c.w, c.h)), Some((85, 50)));
        assert_eq!(by_name("m4_vc.xp").map(|c| (c.w, c.h)), Some((13, 27)));
    }
}
