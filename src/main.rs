// Much of the campaign is translated ahead of the screens that call it.
#![allow(dead_code)]

mod bones;
mod calendar;
mod campaign;
mod color;
mod combat;
mod console;
mod credits;
mod crew;
mod day;
mod daymap;
mod defs;
mod draw;
mod encounter;
mod enemy;
mod game;
mod glyph;
mod layout;
mod model;
mod rng;
mod screens;
mod tank;
mod title;
mod ui;
mod xp;

use console::Console;
use game::Game;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use rng::Rng;
use std::io;
use std::time::{Duration, Instant};
use title::{Choice, MainMenu};
use ui::Screen;

const HELP: &str = "armcom [--art | --encounter [advance|battle|counterattack]]

Armoured Commander, the World War II tank commander roguelike by Gregory
Adam Scott, on its way to the terminal. New Campaign plays a whole
campaign; there are no saved games yet, and no sound.

  --art        browse the game's REXPaint art (tank diagrams, campaign maps);
               left/right to change picture, q to quit
  --encounter  go straight into one encounter with a fresh crew, for
               trying the battle rules (default: advance)

Needs a terminal of at least 100x30.
Session log: RUST_LOG=debug armcom 2>armcom.log";

const FRAME: Duration = Duration::from_millis(20);
const SNAPSHOT_EVERY: Duration = Duration::from_secs(1);
const NOTE_FOR: Duration = Duration::from_secs(3);

enum Mode {
    Menu(Box<MainMenu>),
    Credits(Box<credits::Credits>),
    Scores(Box<Console>),
    Art { index: usize, con: Console },
}

impl Mode {
    fn name(&self) -> String {
        match self {
            Mode::Menu(_) => "menu".into(),
            Mode::Credits(_) => "credits".into(),
            Mode::Scores(_) => "high scores".into(),
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

    if let Some(n) = args.iter().position(|a| a == "--encounter") {
        let scen = match args.get(n + 1).map(String::as_str) {
            Some("battle") => "Battle",
            Some("counterattack") => "Counterattack",
            _ => "Advance",
        };
        log::info!("start: encounter ({scen})");
        let terminal = ratatui::init();
        let _ = ratatui::crossterm::execute!(io::stdout(), event::EnableMouseCapture);
        let mut game = Game::new(Screen::new(Some(terminal)), rng);
        game.debug_encounter(scen);
        ui::quit();
    }

    let mode = if args.iter().any(|a| a == "--art") {
        art(0)
    } else {
        Mode::Menu(Box::new(MainMenu::new(&mut rng)))
    };
    log::info!("start: {}", mode.name());

    let terminal = ratatui::init();
    let result = run(terminal, mode, &mut rng);
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

fn run(mut terminal: ratatui::DefaultTerminal, mut mode: Mode, rng: &mut Rng) -> io::Result<()> {
    let mut note: Option<(String, Instant)> = None;
    let mut snapshot = Instant::now();
    loop {
        match &mut mode {
            Mode::Menu(menu) => {
                menu.tick(rng);
            }
            Mode::Credits(c) => {
                c.tick();
            }
            _ => {}
        }
        if note.as_ref().is_some_and(|(_, at)| at.elapsed() > NOTE_FOR) {
            note = None;
        }
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();
            let (con, caption) = match &mode {
                Mode::Menu(menu) => (&menu.con, note.as_ref().map(|(n, _)| n.clone())),
                Mode::Credits(c) => (&c.con, None),
                Mode::Scores(c) => (&**c, None),
                Mode::Art { index, con } => {
                    let (name, _) = xp::FILES[*index];
                    let caption = format!(
                        "{name}  {}x{}  ({}/{})  left/right: next picture, q: quit",
                        con.w,
                        con.h,
                        index + 1,
                        xp::FILES.len()
                    );
                    (con, Some(caption))
                }
            };
            ui::present(con, area, buf);
            if let Some(text) = caption {
                let y = area.y + area.height.saturating_sub(1);
                for (i, ch) in text.chars().take(area.width as usize).enumerate() {
                    buf[(area.x + i as u16, y)]
                        .set_char(ch)
                        .set_fg(ratatui::style::Color::Black)
                        .set_bg(ratatui::style::Color::Rgb(0, 200, 255));
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
                    Some(Choice::NewCampaign) => {
                        log::info!("switch: new campaign");
                        terminal = play_campaign(terminal)?;
                        mode = Mode::Menu(Box::new(MainMenu::new(rng)));
                        log::info!("switch: menu");
                        continue;
                    }
                    Some(Choice::HighScores) => {
                        mode = Mode::Scores(Box::new(credits::high_scores()));
                        log::info!("switch: high scores");
                        continue;
                    }
                    Some(Choice::Credits) => {
                        mode = Mode::Credits(Box::new(credits::Credits::new()));
                        log::info!("switch: credits");
                        continue;
                    }
                    None => {}
                }
                if let Some((n, _)) = &note {
                    log::info!("menu: {n}");
                }
            }
            Mode::Scores(_) => {
                if key.code == KeyCode::Enter {
                    mode = Mode::Menu(Box::new(MainMenu::new(rng)));
                    log::info!("switch: menu");
                }
            }
            Mode::Credits(c) => match key.code {
                KeyCode::Enter | KeyCode::Esc => {
                    mode = Mode::Menu(Box::new(MainMenu::new(rng)));
                    log::info!("switch: menu");
                }
                KeyCode::Char('p') | KeyCode::Char('P') => c.toggle_pause(),
                _ => {}
            },
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

/// A new campaign, from the main menu until it ends or the player quits;
/// the terminal is the campaign's while it runs, with the mouse on.
fn play_campaign(terminal: ratatui::DefaultTerminal) -> io::Result<ratatui::DefaultTerminal> {
    let _ = ratatui::crossterm::execute!(io::stdout(), event::EnableMouseCapture);
    let mut game = Game::new(Screen::new(Some(terminal)), Rng::from_clock());
    game.new_campaign();
    let mut terminal = game.scr.into_terminal().expect("the campaign hands the terminal back");
    let _ = ratatui::crossterm::execute!(io::stdout(), event::DisableMouseCapture);
    terminal.clear()?;
    Ok(terminal)
}
