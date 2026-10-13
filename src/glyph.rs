//! Glyph codes to Unicode.
//!
//! ArmCom draws with libtcod's code page 437 layout through its own font,
//! `terminal8x12_armcom.png`. Most of the font is plain CP437; where it
//! differs, the table follows the font's bitmaps, checked glyph by glyph:
//! the single-line corners are rounded, and 224 to 232 (Greek letters in
//! CP437) are triangles and quadrant blocks the tank and map art is drawn
//! with.

const TABLE: [char; 256] = [
    // 0x00: control codes, drawn as CP437's symbols (0 is blank, 7 a disc)
    ' ', '☺', '☻', '♥', '♦', '♣', '♠', '●', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼',
    '►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
    // 0x20: ASCII
    ' ', '!', '"', '#', '$', '%', '&', '\'', '(', ')', '*', '+', ',', '-', '.', '/',
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
    '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
    'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\', ']', '^', '_',
    '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
    'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '⌂',
    // 0x80: accented letters and symbols
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0: shades and box drawing; 191, 192, 217 and 218 are rounded in the font
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '╮',
    '╰', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '╯', '╭', '█', '▄', '▌', '▐', '▀',
    // 0xE0: the font's triangles and quadrants, then CP437 again from 233
    '◤', '◥', '▘', '▝', '▀', '▗', '▚', '▐', '▖', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
];

pub fn to_char(code: u8) -> char {
    TABLE[code as usize]
}

/// libtcod's named characters that ArmCom uses.
#[allow(dead_code)]
pub mod named {
    pub const HEART: u8 = 3;
    pub const SPADE: u8 = 6;
    pub const BULLET: u8 = 7;
    pub const BULLET_INV: u8 = 8;
    pub const RADIO_UNSET: u8 = 9;
    pub const RADIO_SET: u8 = 10;
    pub const ARROW2_N: u8 = 30;
    pub const DVLINE: u8 = 186;
    pub const BLOCK1: u8 = 176;
    pub const BULLET_SQUARE: u8 = 254;
}

/// Text to glyph codes. ArmCom builds strings with `chr(n)` for glyphs
/// above 127 (`chr(248)` for degrees), and its Python 2 strings reach
/// libtcod as those bytes, so a character below 256 is its own code;
/// anything else becomes `?`.
pub fn encode(text: &str) -> Vec<u8> {
    text.chars().map(|c| if (c as u32) < 256 { c as u32 as u8 } else { b'?' }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_is_itself() {
        for c in 0x20u8..0x7f {
            assert_eq!(to_char(c), c as char);
        }
    }

    #[test]
    fn font_overrides() {
        assert_eq!(to_char(191), '╮');
        assert_eq!(to_char(219), '█');
        assert_eq!(to_char(228), '▀');
        assert_eq!(to_char(232), '▖');
    }
}
