# Armoured Commander

A Rust + Ratatui port of Armoured Commander, Gregory Adam Scott's World War
II tank commander roguelike (2015 to 2017, maintained by Eric Normandeau
from 1.04), for the Asciiverse catalog. The original is already made of
character cells; it draws them into its own SDL window through libtcod.
The port keeps the game and puts the cells in the terminal.

Status: started 2026-10-13. Console layer, glyph table, `.xp` art and the
animated main menu work; the campaign is not translated yet.

## Source and licence

- Original: github.com/sudasana/armcom, last commit `ca6a1cf`
  (2020-12-26, version 1.0 subversion 9), cloned at
  `../../references/Armoured Commander`. Python 2.7 and libtcod, about
  12,300 lines of code without comments and blank lines (`armcom.py`
  10,400, `armcom_defs.py` and `armcom_vehicle_defs.py` 1,900).
- GPL-3.0-or-later for the game and its data, so this port is GPL too
  (`LICENSE`, copied from the original). `xp_loader.py` was MIT (Sean
  Hagar); `src/xp.rs` is our own reader of the same format.
- The art in `data/` is the original's REXPaint files, unchanged.
- Sounds (`sounds/*.wav` in the original, GPL with the rest) are not
  brought over yet.
- Steamworks is dropped.
- The name stays "Armoured Commander" (David, 2026-10-18), with Gregory
  Adam Scott's blessing to be asked for (sudasana@gmail.com, in the
  original's copyright lines); to be asked after the Asciiverse launch,
  so there is something to show him (David, 2026-10-18).
- The sequel, Armoured Commander II, is cloned at
  `../../references/Armoured Commander II` (GPL 3.0.2, 2020). It is a
  separate title for later; its newer Steam versions are not GPL data.

## How the port is built

The original draws everything with about 40 libtcod console functions.
`src/console.rs` is that console model (glyph codes 0 to 255, foreground
and background, offscreen consoles, blit with a key colour, centred print
with colour controls), so the Python's drawing code translates line for
line. Only `present` in `main.rs` knows about the terminal.

- `src/glyph.rs`: code to Unicode. The original font
  (`terminal8x12_armcom.png`) is CP437 except rounded single-line corners
  (191, 192, 217, 218) and 224 to 232, which are triangles and quadrant
  blocks instead of Greek letters. Checked against the font's bitmaps.
- `src/xp.rs`: the `.xp` files are built into the binary.
- `src/color.rs`: libtcod's named colours (values from the shipped
  libtcodpy) and `color_lerp`.
- `src/rng.rs`: the original rolls with both libtcod's generator and
  Python's `random`, clock-seeded, so there is no sequence to match; the
  port keeps each roll's range and odds.
- `src/defs.rs`: the definitions. `tools/defs2json.py` runs the
  original's `armcom_defs.py` and `armcom_vehicle_defs.py` against a stub
  libtcod and writes `data/defs.json` and `data/vehicles.json`; rerun it
  rather than editing the JSON. Vehicle stats stay a keyed list, since
  some are flags that matter by being present (`'HVSS' in stats`). The
  campaign XML files are shipped unchanged and read by `LoadCampaignInfo`'s
  rules. One quirk kept: Patton's Best lists `M4A3E2(76)W Turret G`, which
  has no vehicle entry, so `GetRF` gives it rarity 0 and it is never offered.
- `src/title.rs`: `MainMenu`, ported line for line (no bones file or saved
  game yet, so no tombstone and no Continue).

`armcom --art` browses the art; `RUST_LOG=debug armcom 2>armcom.log` logs
every key, mode change and a state line each second.

## Screen size

The original screen is 149x61 cells of 8x12 pixels. Agreed with David
2026-10-18:

- **Every screen is laid out for 100x30** (the catalog preview size) as
  the minimum, and uses extra room on bigger terminals (a taller message
  log, for instance).
- **Encounter:** a 40-column left panel (date, tank, crew and orders,
  phase prompt, messages, hex info) and the hex map on the right. The 37
  hexes are redrawn at 8 columns across and 4 rows down (the original:
  9 and 6), 58x29 in all.
- **Campaign day map:** generated at terminal proportions, since
  `GenerateCampaignMap` builds it fresh each day; only the map's shape
  changes, not the rules. Scrolling window beside a 40-column panel.
- The overview maps (85x50, nearly all solid blocks) show in half blocks
  at 85x25. Menus become popups sized to their content.

Notes from the first look:

- Much of the big art is solid colour, not text: the title's sky and
  letters, and the campaign maps (85x50, nearly all glyph 219). Half
  blocks draw two of those rows per terminal row.
- The game is already split into fixed panels (`*_CON_WIDTH`,
  `*_CON_HEIGHT` in armcom.py: tank 73x37, messages 73x19, encounter map
  73x51, campaign map window 90x57), so a new layout moves and resizes
  panels while their contents translate as they are.

## Order of work

1. Done: console layer, glyphs, art, main menu.
2. Done: terminal layout agreed (see Screen size).
3. Translate the data (`armcom_defs.py`, `armcom_vehicle_defs.py`, the
   campaign XML) and the rules, in the order `armcom.py` reads.
4. Saves, bones file (graveyard, high scores), sound.

## House conventions

- Session logging (`log` + `env_logger`, see `../../CLAUDE.md`).
- Saves go to the platform's data folder, not the install folder.
- Release builds by the same GitHub workflow as Greed
  (`../greed/.github/workflows/release.yml`).
