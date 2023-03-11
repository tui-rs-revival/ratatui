use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{
        Block, Borders, Cell, InteractiveWidgetState, List, ListItem, Paragraph, Row, Table, TextInput,
        TextInputState,
    },
    Frame, Terminal,
};

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

const NUM_INPUTS: usize = 3;

#[derive(Default)]
struct App {
    input_states: [TextInputState; NUM_INPUTS],
    focused_input_idx: Option<usize>,
    events: Vec<Event>,
}

impl App {
    fn focus_next(&mut self) {
        self.focused_input_idx = match self.focused_input_idx {
            Some(idx) => {
                if idx == (NUM_INPUTS - 1) {
                    None
                } else {
                    Some(idx + 1)
                }
            }
            None => Some(0),
        };

        self.set_focused();
    }

    fn focus_prev(&mut self) {
        self.focused_input_idx = match self.focused_input_idx {
            Some(idx) => {
                if idx == 0 {
                    None
                } else {
                    Some(idx - 1)
                }
            }
            None => Some(NUM_INPUTS - 1),
        };

        self.set_focused();
    }

    fn set_focused(&mut self) {
        for input_state in self.input_states.iter_mut() {
            input_state.unfocus();
        }

        if let Some(idx) = self.focused_input_idx {
            self.input_states[idx].focus();
        }
    }

    fn focused_input_mut(&mut self) -> Option<&mut TextInputState> {
        if let Some(idx) = self.focused_input_idx {
            Some(&mut self.input_states[idx])
        } else {
            None
        }
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut app = App::default();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let event = event::read()?;
        app.events.push(event);

        if let Some(state) = app.focused_input_mut() {
            if state.handle_event(event).is_consumed() {
                continue;
            }
        }

        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Tab => app.focus_next(),
                KeyCode::BackTab => app.focus_prev(),
                _ => {}
            },
            _ => {}
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &mut App) {
    let layout = Layout::default()
        .horizontal_margin(10)
        .vertical_margin(2)
        .constraints(
            [
                Constraint::Length(10),
                Constraint::Length(14),
                Constraint::Length(5),
                Constraint::Percentage(100),
            ]
            .as_ref(),
        )
        .split(f.size());

    let info_block = Paragraph::new(vec![
        Spans::from(Span::raw("Press 'TAB' to go to the next input")),
        Spans::from(Span::raw("Press 'SHIFT+TAB' to go to the previous input")),
        Spans::from(Span::raw("Press 'q' to quit when no input is focused")),
        Spans::from(Span::raw(
            "Supports a subset of readline keyboard shortcuts:",
        )),
        Spans::from(Span::raw(
            " - ctrl+e / ctrl+a to jump to text input end / start",
        )),
        Spans::from(Span::raw(
            " - ctrl+w delete to the start of the current word",
        )),
        Spans::from(Span::raw(
            " - alt+b / alt+f to jump backwards / forwards a word",
        )),
        Spans::from(Span::raw(" - left / right arrow keys to move the cursor")),
    ])
    .block(Block::default().title("Information").borders(Borders::ALL));
    f.render_widget(info_block, layout[0]);

    let inputs_block = Block::default().title("Inputs").borders(Borders::ALL);
    let inputs_rect = inputs_block.inner(layout[1]);
    f.render_widget(inputs_block, layout[1]);

    let inputs_layout = Layout::default()
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(inputs_rect);

    {
        let text_input =
            TextInput::new().block(Block::default().title("Basic Input").borders(Borders::ALL));
        f.render_interactive(text_input, inputs_layout[0], &mut app.input_states[0]);
    }
    {
        let text_input = TextInput::new()
            .block(
                Block::default()
                    .title("Has Placeholder")
                    .borders(Borders::ALL),
            )
            .placeholder_text("Type something...");
        f.render_interactive(text_input, inputs_layout[1], &mut app.input_states[1]);
    }
    {
        let text_input = TextInput::new()
            .text_style(Style::default().fg(Color::Yellow))
            .block(Block::default().title("Is Followed").borders(Borders::ALL));
        f.render_interactive(text_input, inputs_layout[2], &mut app.input_states[2]);
    }
    {
        let text_input = TextInput::new()
            .read_only(true)
            .text_style(Style::default().fg(Color::LightBlue))
            .block(
                Block::default()
                    .title("Follows Above (read only)")
                    .borders(Borders::ALL),
            );
        f.render_interactive(text_input, inputs_layout[3], &mut app.input_states[2]);
    }

    let table = Table::new(
        app.input_states
            .iter()
            .enumerate()
            .map(|(idx, input_state)| {
                Row::new(vec![
                    Cell::from(Span::raw(format!("Input {}", idx + 1))),
                    Cell::from(Span::styled(
                        input_state.get_value(),
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
                ])
            })
            .collect::<Vec<_>>(),
    )
    .widths(&[Constraint::Min(10), Constraint::Percentage(100)])
    .block(Block::default().title("Input Values").borders(Borders::ALL));
    f.render_widget(table, layout[2]);

    let events = List::new(
        app.events
            .iter()
            .rev()
            .map(|event| ListItem::new(Span::raw(format!("{:?}", event))))
            .collect::<Vec<_>>(),
    )
    .block(Block::default().title("Events").borders(Borders::ALL));
    f.render_widget(events, layout[3]);
}
