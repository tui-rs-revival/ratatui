mod util;

use crate::util::event::{Event, Events};
use std::{error::Error, io};
use termion::{input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};

use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, MultiListState, MutliList};
use tui::{backend::TermionBackend, widgets::ListItem, Terminal};

fn main() -> Result<(), Box<dyn Error>> {
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::new();

    let mut state = MultiListState::default();
    state.highlight(Some(0));
    let items: Vec<ListItem> = vec![
        ListItem::new("Option 1"),
        ListItem::new("Option 2"),
        ListItem::new("Option 3"),
    ];

    loop {
        match events.next().unwrap() {
            Event::Input(i) => match i {
                termion::event::Key::Left => {
                    if let Some(h) = state.get_highlight() {
                        state.deselect(h);
                    }
                }
                termion::event::Key::Right => {
                    if let Some(h) = state.get_highlight() {
                        state.select(h);
                    }
                }
                termion::event::Key::Up => {
                    let mut h = state.get_highlight().unwrap_or(0);
                    h = if h == 0 { items.len() - 1 } else { h - 1 };

                    state.highlight(Some(h));
                }
                termion::event::Key::Down => {
                    let mut h = state.get_highlight().unwrap_or(0);
                    h = if h >= items.len() - 1 { 0 } else { h + 1 };

                    state.highlight(Some(h));
                }
                termion::event::Key::Char(c) => match c {
                    '\n' => {
                        if let Some(h) = state.get_highlight() {
                            state.toggle_selection(h);
                        }
                    }
                    _ => (),
                },
                termion::event::Key::Ctrl(c) => {
                    if c == 'c' {
                        break;
                    }
                }
                _ => (),
            },
            Event::Tick => (),
        }
        terminal.draw(|f| {
            let chunk = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(f.size())[0];

            let multilist = MutliList::new(items.clone())
                .block(Block::default().borders(Borders::ALL))
                .selected_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol(">>");

            f.render_stateful_widget(multilist, chunk, &mut state);
        })?;
    }

    Ok(())
}
