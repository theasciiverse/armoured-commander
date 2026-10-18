mod color;
mod console;
#[allow(dead_code)] // read by the rules as they are translated
mod defs;
mod glyph;
mod rng;
mod title;
mod xp;

use console::Console;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::Color as TuiColor;
use rng::Rng;
use std::io;
use std::time::{Duration, Instant};
use title::{Choice, MainMenu};

const HELP: &str = "armcom [--art]

Armoured Commander, the World War II tank commander roguelike by Gregory
Adam Scott, on its way to the terminal. Early port: the main menu runs,
the campaign does not yet.

  --art     browse the game's REXPaint art (tank diagrams, campaign maps);
            left/right to change picture, q to quit

The original screen is 149x61; a smaller terminal shows its middle.
Session log: RUST_LOG=debug armcom 2>armcom.log";

const FRAME: Duration = Duration::from_millis(20);
const SNAPSHOT_EVERY: Duration = Duration::from_secs(1);
const NOTE_FOR: Duration = Duration::from_secs(3);

enum Mode {
    Menu(Box<MainMenu>),
    Art { index: usize, con: Console },
}

impl Mode {
    fn name(&self) -> String {
        match self {
            Mode::Menu(_) => "menu".into(),
            Mode::Art { index, .. } => format!("art {}", xp::FILES[*index].0),
        }
    }
}

fn main() -> io::Result<()> {
    env_logger::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(());
    }
    let mut rng = Rng::from_clock();
    let mode = if args.iter().any(|a| a == "--art") {
        art(0)
    } else {
        Mode::Menu(Box::new(MainMenu::new(&mut rng)))
    };
    log::info!("start: {}", mode.name());

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, mode, &mut rng);
    ratatui::restore();
    log::info!("quit");
    result
}

fn art(index: usize) -> Mode {
    let (name, data) = xp::FILES[index];
    let con = xp::load(data).unwrap_or_else(|e| {
        log::error!("{name}: {e}");
        Console::new(1, 1)
    });
    Mode::Art { index, con }
}

fn run(terminal: &mut ratatui::DefaultTerminal, mut mode: Mode, rng: &mut Rng) -> io::Result<()> {
    let mut note: Option<(String, Instant)> = None;
    let mut snapshot = Instant::now();
    loop {
        if let Mode::Menu(menu) = &mut mode {
            menu.tick(rng);
        }
        if note.as_ref().is_some_and(|(_, at)| at.elapsed() > NOTE_FOR) {
            note = None;
        }
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            match &mode {
                Mode::Menu(menu) => present(&menu.con, area, buf, note.as_ref().map(|(n, _)| n.as_str())),
                Mode::Art { index, con } => {
                    let (name, _) = xp::FILES[*index];
                    let caption = format!(
                        "{name}  {}x{}  ({}/{})  left/right: next picture, q: quit",
                        con.w,
                        con.h,
                        index + 1,
                        xp::FILES.len()
                    );
                    present(con, area, buf, Some(&caption));
                }
            }
        })?;

        if snapshot.elapsed() >= SNAPSHOT_EVERY {
            let size = terminal.size()?;
            log::debug!("state: mode={} terminal={}x{}", mode.name(), size.width, size.height);
            snapshot = Instant::now();
        }

        if !event::poll(FRAME)? {
            continue;
        }
        let ev = event::read()?;
        log::debug!("input: {ev:?}");
        let Event::Key(key) = ev else { continue };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Ok(());
        }
        match &mut mode {
            Mode::Menu(menu) => {
                let KeyCode::Char(c) = key.code else { continue };
                match menu.key(c) {
                    Some(Choice::Quit) => return Ok(()),
                    Some(Choice::NewCampaign) => note = Some(("New Campaign is not ported yet".into(), Instant::now())),
                    Some(Choice::HighScores) => note = Some(("High Scores are not ported yet".into(), Instant::now())),
                    Some(Choice::Credits) => note = Some(("Credits are not ported yet".into(), Instant::now())),
                    None => {}
                }
                if let Some((n, _)) = &note {
                    log::info!("menu: {n}");
                }
            }
            Mode::Art { index, .. } => {
                let n = xp::FILES.len();
                let next = match key.code {
                    KeyCode::Right | KeyCode::Char(' ') => (*index + 1) % n,
                    KeyCode::Left => (*index + n - 1) % n,
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    _ => continue,
                };
                mode = art(next);
                log::info!("switch: {}", mode.name());
            }
        }
    }
}

/// Draw a console centred in the terminal, its middle if it does not fit,
/// with an optional line of text along the bottom.
fn present(con: &Console, area: Rect, buf: &mut Buffer, caption: Option<&str>) {
    let (aw, ah) = (area.width as i32, area.height as i32);
    // offset of the console's (0, 0) on screen; negative when cropped
    let ox = (aw - con.w) / 2;
    let oy = (ah - con.h) / 2;
    for sy in 0..ah {
        for sx in 0..aw {
            let cell = &mut buf[(area.x + sx as u16, area.y + sy as u16)];
            match con.cell(sx - ox, sy - oy) {
                Some(c) => {
                    cell.set_char(glyph::to_char(c.ch)).set_fg(rgb(c.fg)).set_bg(rgb(c.bg));
                }
                None => {
                    cell.set_char(' ').set_fg(TuiColor::Reset).set_bg(TuiColor::Black);
                }
            }
        }
    }
    let fits = con.w <= aw && con.h <= ah;
    let mut line = caption.map(str::to_string);
    if !fits && line.is_none() {
        line = Some(format!(
            "Terminal is {aw}x{ah}; the original screen is {}x{}, so this is its middle",
            con.w, con.h
        ));
    }
    if let Some(text) = line {
        let y = area.y + area.height.saturating_sub(1);
        for (i, ch) in text.chars().take(aw as usize).enumerate() {
            buf[(area.x + i as u16, y)]
                .set_char(ch)
                .set_fg(TuiColor::Black)
                .set_bg(TuiColor::Rgb(0, 200, 255));
        }
    }
}

fn rgb(c: color::Color) -> TuiColor {
    TuiColor::Rgb(c.0, c.1, c.2)
}
