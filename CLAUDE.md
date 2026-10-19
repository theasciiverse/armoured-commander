# Armoured Commander

A Rust + Ratatui port of Armoured Commander, Gregory Adam Scott's World War
II tank commander roguelike (2015 to 2017, maintained by Eric Normandeau
from 1.04), for the Asciiverse catalog. The original is already made of
character cells; it draws them into its own SDL window through libtcod.
The port keeps the game and puts the cells in the terminal.

Status: started 2026-10-13. Console layer, glyph table, `.xp` art, the
animated main menu, and (2026-10-18) the whole encounter: battle rules,
enemy units, crew, and the 100x30 encounter screen. `armcom --encounter
[advance|battle|counterattack]` plays one encounter with a fresh crew.
Also ported: credits, high scores, F1 help, F2 tank info, F4 settings
(without the full screen options), F5 campaign stats and the journal.
2026-10-19: the campaign. New Campaign on the main menu plays a whole
campaign: choosing it and its settings, the crew and their skills (F3
crew info too), the calendar with refits and tank offers, the day map
with its actions, quests and random events, battles, crew replacement,
awards and promotions, the end of the campaign with its high score and
journal file. Saved games (2026-10-19): saved at the original's points,
including mid-battle, and picked up again by Continue on the main menu.
The main menu (2026-10-19) is laid out for the terminal too, with its
tombstone. Bigger encounter hexes on big screens (2026-10-19). Not yet:
sound.

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
  Every decision made here is to be looked at for reuse in II (David,
  2026-10-18): keep the console layer, glyph table, `.xp` reader, data
  loading and screen-layout approach free of ArmCom I specifics where
  that costs nothing, and note here what II could take over.

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
- `src/title.rs`: `MainMenu`, ported line for line, with Continue and a
  tombstone from the graveyard. Its 30-row scene (sky, clouds, hills,
  title) is drawn in half blocks at the height there is room for: 15 rows
  at 100x30, its own 30 from 45 rows up; the text goes below, the
  tombstone bottom right (nearer the edge on narrow screens, clear of the
  copyright lines). The confirmation before New Campaign replaces a save
  is a box over the scene. Rebuilt on a resize.
- The campaign: `src/calendar.rs` (new campaign, calendar, refits, awards,
  the campaign's end), `src/day.rs` (a day of action: its actions,
  `PostEncounter`, the tank view and ammunition menu, the day loop),
  `src/daymap.rs` (`GetPath`, map generation and painting, the day
  screen), `src/campaign.rs` (the `Campaign` class's methods: weather,
  time, enemy advances, random events, sunset). F3 crew info and skills
  are in `src/screens.rs`. `main.rs` lends the terminal to a `Game` for
  the length of a campaign and takes it back for the menu.
- `src/save.rs`: the saved game, `savegame.json` in the data folder: the
  campaign, tank and battle as serde JSON (the original shelved the same
  three objects), with `SavedGameInfo` for the main menu. What the
  campaign file provides (calendar, ranks, activation tables) is not
  saved but read again on loading, as `LoadGame` does. `save::FORMAT` is
  bumped whenever the saved state changes shape; the menu then says the
  save cannot be loaded instead of loading it wrong. One save at a time,
  deleted when the campaign ends, as in the original.

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
  Done (2026-10-19) at 59x82 for the original's 90x90, in a 59x29 window:
  the height is what keeps a day as long. Run 300 times at each size, the
  generator crosses 7.1 areas from start to exit at 90x90 and 7.0 at
  59x82, with about 40 areas on the map instead of 55. The left panel has
  the date, the actions (their names shortened to fit 39 columns) and the
  info panel, which describes the area under the pointer or, with no
  pointer over the map, the selected or current area (the original needed
  the mouse for this).
- The overview maps (85x50, nearly all solid blocks) show in half blocks
  at 85x25. Menus become popups sized to their content. As built: the
  calendar needs its 40-column text panel too, so the half-block map shows
  in a 59-column window that follows the day's location marker, rather
  than all 85 columns. For David to look at.
- **Bigger terminals** (2026-10-19, David asked for more of the
  terminal): the screen takes the terminal's size, at least 100x30, and
  is laid out again when the terminal is resized. `layout.rs` has the
  sizes that follow it as functions (`sw()`, `sh()`, `map_con_x()`,
  `msg_con_height()`, ...). The encounter's message log runs the full
  height and the hex map is centred to its right; the day map is
  generated as wide as its window, 59 to 90 columns (always 82 tall), and
  its window fills the right of the screen, scrolling sideways if the
  terminal is later narrowed; the calendar's overview map is scaled to
  fill its space, in half blocks, in the proportions of the original's
  8x12-pixel cells, with the place names at their scaled places (at
  100x30 it stays 85x25, in a window that follows the marker). Windows and
  pop-ups stay 100x30, centred. The encounter's hexes grow with the
  screen: `layout::hex_k()` is the biggest of three sizes that fits, 4
  (59x29 map), 6 (81x43, from about 122x44) or 8 (103x57, from about
  144x58). Hexes are `k` rows apart and `3k/2+2` columns, the spacing at
  which edges that slant a column a row are shared by neighbours. The hex
  outline, the edges between sectors, unit spots inside a hex and the hex
  under the pointer are all worked out from `k`; a battle carried over to
  another size (a resize, a saved game) keeps each unit in its hex.
- Other windows at 100x30: crew panels 19 wide (a rank too long for one,
  "Private First Class", shows short); the ammunition menu packed into 30
  rows. Alt switched the ammunition menu's direction of move in the
  original; a terminal cannot see Alt alone, so Tab does.

Notes from the first look:

- Much of the big art is solid colour, not text: the title's sky and
  letters, and the campaign maps (85x50, nearly all glyph 219). Half
  blocks draw two of those rows per terminal row.
- The game is already split into fixed panels (`*_CON_WIDTH`,
  `*_CON_HEIGHT` in armcom.py: tank 73x37, messages 73x19, encounter map
  73x51, campaign map window 90x57), so a new layout moves and resizes
  panels while their contents translate as they are.

## Bugs in the original

The rule (2026-10-18): fix what only affects display or crashes; keep
what changes the game's odds, and list it here for David to decide.

Fixed: `AttackPlayer` names a heard gun from one character ("7mm");
`GetCharLocation` compares `abs(unit.x + x)`, so units in a hex could
overlap; `GenerateName` meant to avoid duplicate names; `DepleteSmoke`
removed from the list it looped over; HVAP/APDS against an unarmoured
spot crashed (now the AP table, as its comment says); a unit that dies
during its own action no longer acts again; after an ambush knocks out
the tank, the battle is cleared instead of lingering into the campaign.

Fixed in the campaign: on a forest day the terrain table stops at 11, so
a roll of 12 left an area with no terrain, which crashed when captured
(`AwardCaptureVP`); it is marshland now, the table's last entry. In a
counterattack, Tab listed a front-line area once for each enemy area
beside it, and could stick on it. `CheckCommander` cannot end a campaign
twice. The display settings carry over to a new campaign.

Fixed, but it touches the odds, so for David to confirm: the original
builds roads inside `PaintCampaignMap`, which runs twice for every new
map, so most maps got a second network of roads over the first (and
movement along roads is quicker). Roads are built once here.

Kept, to decide (campaign): area resistance is rolled before the roads
exist, so its road modifiers (+2 improved, +1 dirt) never apply.

Kept, to decide: the campaign files' date-based activation modifiers
never apply (`v += mod['mod']` changes a copy); the spawn-range table
tests `'AT'` not `'AT_GUN'`; crew wound modifiers test for `'turret'` in
lower case, so turret hits count as hull hits; the player's gunner's
Target Tracking is rolled when an enemy fires too; `AttackInfantry`
returns nothing after a kill and `AttackPlayer` nothing after a
panzerfaust, so the unit rolls another action; a repair "6 and 2" (likely
meant double six) breaks the part; an ambush that knocks out the tank
counts no VP for that encounter.

## Tests

`cargo test` includes 450 headless encounters (150 seeds, three
missions) played by an autopilot to the end, and six whole campaigns
from New Campaign to the end of the calendar or the commander's death
(random campaign, settings and start date; `ARMCOM_CAMPAIGNS=80 cargo
test --release campaigns` for more, about 35 seconds). All must finish
without a panic. Both campaigns' day maps are checked for a way from
start to exit. Saves taken during headless campaigns (one in a battle,
one during a day, one on the calendar, per campaign) must load back
unchanged and play on to the end. The release build runs it all in
about 10 seconds; debug takes about two minutes.

## Order of work

1. Done: console layer, glyphs, art, main menu.
2. Done: terminal layout agreed (see Screen size).
3. Done: the data, the encounter, the campaign, the info windows.
4. Done: saves, the main menu with its tombstone, bigger hexes. Next:
   sound. The bones file already keeps high scores, the graveyard and
   tutorial flags; journals are written to the data folder at the end of
   a campaign.
5. Then Armoured Commander II (David, 2026-10-19), reusing what applies.

## House conventions

- Session logging (`log` + `env_logger`, see `../../CLAUDE.md`).
- Saves go to the platform's data folder, not the install folder.
- Release builds by the same GitHub workflow as Greed
  (`../greed/.github/workflows/release.yml`).
