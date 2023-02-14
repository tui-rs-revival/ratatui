use crate::{app::App, ui};
use std::{
    error::Error,
    io::{self, Write},
    sync::mpsc,
    thread,
    time::Duration,
};
use termion::{
    event::Key,
    input::{MouseTerminal, TermRead},
    raw::IntoRawMode,
    screen::{ToAlternateScreen, ToMainScreen},
};
use tui::{
    backend::{Backend, TermionBackend},
    Terminal,
};

pub fn run(tick_rate: Duration, enhanced_graphics: bool) -> Result<(), Box<dyn Error>> {
    // setup terminal
    let mut stdout = MouseTerminal::from(io::stdout()).into_raw_mode()?;
    stdout.activate_raw_mode()?;
    write!(stdout, "{}", ToAlternateScreen)?;
    let mut terminal = Terminal::new(TermionBackend::new(stdout))?;

    // create app and run it
    let app = App::new("Termion demo", enhanced_graphics);
    let res = run_app(&mut terminal, app, tick_rate);

    // restore terminal
    terminal.backend().suspend_raw_mode()?;
    write!(terminal.backend_mut(), "{}", ToMainScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    tick_rate: Duration,
) -> Result<(), Box<dyn Error>> {
    let events = events(tick_rate);
    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        match events.recv()? {
            Event::Input(key) => match key {
                Key::Char(c) => app.on_key(c),
                Key::Up => app.on_up(),
                Key::Down => app.on_down(),
                Key::Left => app.on_left(),
                Key::Right => app.on_right(),
                _ => {}
            },
            Event::Tick => app.on_tick(),
        }
        if app.should_quit {
            return Ok(());
        }
    }
}

enum Event {
    Input(Key),
    Tick,
}

fn events(tick_rate: Duration) -> mpsc::Receiver<Event> {
    let (tx, rx) = mpsc::channel();
    let keys_tx = tx.clone();
    thread::spawn(move || {
        let stdin = io::stdin();
        for key in stdin.keys().flatten() {
            if let Err(err) = keys_tx.send(Event::Input(key)) {
                eprintln!("{}", err);
                return;
            }
        }
    });
    thread::spawn(move || loop {
        if let Err(err) = tx.send(Event::Tick) {
            eprintln!("{}", err);
            break;
        }
        thread::sleep(tick_rate);
    });
    rx
}
