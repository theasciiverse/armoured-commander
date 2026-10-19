# Armoured Commander

Gregory Adam Scott's World War II tank commander roguelike (2015 to 2017,
maintained by Eric Normandeau from 1.04), ported to the terminal in Rust and
Ratatui for the Asciiverse catalog. You command a Sherman and its crew of
five through a campaign from the summer of 1944 to the end of the war:
day by day across a map of the front, and battle by battle on a hex map
round the tank, giving each crewman his orders while the dice decide what
every shot does.

The original drew its character cells into its own window through
libtcod. This port keeps the game, its art and its sounds, and draws the
cells in whatever terminal you play in.

## Playing

    armcom

New Campaign plays a whole campaign, saved as it goes; Continue picks it
up again. F1 in the game is the original's help, with every key.

    armcom --encounter [advance|battle|counterattack]

goes straight into one encounter with a fresh crew, for trying the
battle rules.

- The terminal must be at least 100x30. Bigger terminals get a bigger
  map and a longer message log.
- F7 turns sound on and off. On Linux the game uses ALSA (and through it
  PipeWire or PulseAudio) if it is installed, and plays silently if not.
- The saved game, high scores, the graveyard and campaign journals are
  kept in `$XDG_DATA_HOME/armoured-commander`
  (`~/.local/share/armoured-commander`), or `%APPDATA%\armoured-commander`
  on Windows.
- `RUST_LOG=debug armcom 2>armcom.log` writes a session log.

## Changes from the original

The rules, tables, art and sounds are the original's. What is different:

- Laid out for a terminal: every screen fits 100x30 and grows with the
  window; the day map is generated at terminal proportions (the same
  length of day, about 40 areas instead of 55); Tab instead of Alt in the
  ammunition menu.
- Display and crash fixes: names of heard guns, overlapping units in a
  hex, duplicate crew names, smoke that did not clear, HVAP and APDS
  against unarmoured targets, a unit acting again after it died, a
  battle left behind after an ambush, forest terrain rolls of 12, a
  counterattack's front-line list, a campaign that could end twice.
- Fixes that change the odds: roads built once and before resistance is
  rolled, so the road bonus applies; the campaign files' date-based
  enemy activation applies; anti-tank guns use their own range table;
  turret penetrations wound the turret crew; the gunner's Target Tracking
  only helps his own fire; a repair breaks the part on a double six; an
  ambush that knocks out the tank counts that encounter's VP.
- Steam is gone, and so is the screenshot key.

## Credits and licence

- Armoured Commander: Gregory Adam Scott, 2015 to 2017,
  [github.com/sudasana/armcom](https://github.com/sudasana/armcom);
  maintained by Eric Normandeau from 1.04
- Terminal port: Asciiverse
- GPL-3.0-or-later, as the original (see `LICENSE`). The art is the
  original's REXPaint files and the sounds its WAV files, unchanged.
