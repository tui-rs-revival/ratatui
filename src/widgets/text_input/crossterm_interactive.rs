use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::widgets::{InteractiveWidgetState, InteractionOutcome, TextInputState};

impl InteractiveWidgetState for TextInputState {
    fn handle_event(&mut self, event: Event) -> InteractionOutcome {
        if !self.is_focused() {
            return InteractionOutcome::Bubble;
        }

        match event {
            Event::Key(key) => self.handle_key(key),
            _ => InteractionOutcome::Bubble,
        }
    }

    fn is_focused(&self) -> bool {
        self.is_focused()
    }

    fn focus(&mut self) {
        self.focus()
    }

    fn unfocus(&mut self) {
        self.unfocus()
    }
}

impl TextInputState {
    // used in tests
    #[allow(dead_code)]
    fn up_to_cursor(&self) -> &str {
        &self.value[0..self.cursor_pos as usize]
    }

    fn handle_key(&mut self, key: KeyEvent) -> InteractionOutcome {
        if key.modifiers == KeyModifiers::ALT || key.modifiers == KeyModifiers::CONTROL {
            self.handle_modifiers(key.modifiers, key.code)
        } else {
            self.handle_plain(key.code)
        }
    }

    fn word_boundary_idx_under_cursor(&self, scan_backwards: bool) -> usize {
        let value_as_chars = self.get_value().chars().collect::<Vec<_>>();
        let mut char_pairs: Vec<(usize, &[char])> = value_as_chars
            .windows(2) // work in doubles
            .enumerate() // idx of the first char
            .collect();

        if scan_backwards {
            char_pairs = char_pairs
                .into_iter()
                .take(self.cursor_pos.saturating_sub(1))
                .rev()
                .collect();
        } else {
            char_pairs = char_pairs.into_iter().skip(self.cursor_pos).collect()
        }

        if let Some((idx, _chars)) = char_pairs.iter().find(|(_, chars)| {
            // find a boundary where we go from non-whitespace to whitespace
            match (chars[0].is_whitespace(), chars[1].is_whitespace()) {
                (true, true) => false,
                (true, false) => scan_backwards,
                (false, true) => !scan_backwards,
                (false, false) => false,
            }
        }) {
            // println!("bounry at {}: '{}{}'", idx, _chars[0], _chars[1]);
            if scan_backwards {
                idx + 1
            } else {
                idx + 2
            }
        } else {
            // no whitespace boundary found, remove to start of string
            if scan_backwards {
                0
            } else {
                self.value.len()
            }
        }
    }

    fn handle_modifiers(&mut self, modifiers: KeyModifiers, code: KeyCode) -> InteractionOutcome {
        match (modifiers, code) {
            // delete to current word start
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                // find the first boundary going from non-whitespace to whitespace,
                // going backwards from the cursor position
                // println!("up to cursor ({}): '{}'", self.cursor_pos, self.up_to_cursor());

                let remove_to = self.cursor_pos as usize;
                let remove_from = self.word_boundary_idx_under_cursor(true);

                // println!("removing span '{}'", &self.value.as_str()[remove_from..remove_to]);

                // and collect everything that isn't between [remove_from..remove_to)
                self.cursor_pos = remove_from;
                self.value = self
                    .value
                    .chars()
                    .take(remove_from)
                    .chain(self.value.chars().skip(remove_to))
                    .collect();
            }
            // jump to end of line
            (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.cursor_pos = self.value.len();
            }
            // jump to start of line
            (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.cursor_pos = 0;
            }
            // jump back a word
            (KeyModifiers::ALT, KeyCode::Char('b')) => {
                self.cursor_pos = self.word_boundary_idx_under_cursor(true);
            }
            // jump forward a word
            (KeyModifiers::ALT, KeyCode::Char('f')) => {
                self.cursor_pos = self.word_boundary_idx_under_cursor(false);
            }
            _ => return InteractionOutcome::Bubble,
        }
        InteractionOutcome::Consumed
    }

    fn handle_plain(&mut self, code: KeyCode) -> InteractionOutcome {
        match code {
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.value.remove(self.cursor_pos as usize);
                }
            }
            KeyCode::Char(c) => {
                self.value.insert(self.cursor_pos as usize, c);
                self.cursor_pos += 1;
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_pos < self.value.len() {
                    self.cursor_pos += 1;
                }
            }
            _ => return InteractionOutcome::Bubble,
        };

        InteractionOutcome::Consumed
    }
}

#[cfg(test)]
mod test {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::widgets::{InteractiveWidgetState, InteractionOutcome, TextInputState};

    macro_rules! assert_consumed {
        ($expr:expr) => {
            assert_eq!(InteractionOutcome::Consumed, $expr)
        };
    }

    #[test]
    fn test_basics() {
        let mut state = TextInputState::default();

        // don't change when not focused
        assert_eq!(InteractionOutcome::Bubble, state.handle_event(plain('a')));
        assert_eq!("", state.get_value());
        assert_eq!(0, state.cursor_pos);

        state.focus();
        assert_consumed!(state.handle_event(code(KeyCode::Left)));
        assert_eq!(0, state.cursor_pos);
        assert_consumed!(state.handle_event(code(KeyCode::Right)));
        assert_eq!(0, state.cursor_pos);

        assert_consumed!(state.handle_event(plain('a')));
        assert_eq!("a", state.get_value());
        assert_eq!(1, state.cursor_pos);

        // build up a multi-char value
        state.handle_event(plain('s'));
        state.handle_event(plain('d'));
        state.handle_event(plain('f'));
        assert_eq!("asdf", state.get_value());
        assert_eq!(4, state.cursor_pos);

        // remove from end
        state.handle_event(bksp());
        assert_eq!("asd", state.get_value());
        assert_eq!(3, state.cursor_pos);

        // move cursor to middle
        assert_eq!("asd", state.up_to_cursor());
        state.handle_event(code(KeyCode::Left));
        assert_eq!("as", state.up_to_cursor());
        assert_eq!(2, state.cursor_pos);
        assert_eq!("asd", state.get_value());

        // remove from middle
        state.handle_event(bksp());
        assert_eq!(1, state.cursor_pos);
        assert_eq!("ad", state.get_value());
    }

    #[test]
    fn test_ctrl_w_works() {
        let mut state = TextInputState::default();
        state.focus();

        // ctrl+w word removal, from the end of a word
        state.set_value("foo bar baz   smaz");
        state.set_cursor(18);
        assert_consumed!(state.handle_event(ctrl('w')));
        assert_eq!("foo bar baz   ", state.get_value());
        assert_eq!(14, state.cursor_pos);

        // remove runs of trailing whitespace + word
        state.handle_event(ctrl('w'));
        assert_eq!("foo bar ", state.get_value());
        assert_eq!(8, state.cursor_pos);

        // remove from middle of word
        state.handle_event(code(KeyCode::Left));
        state.handle_event(code(KeyCode::Left));
        assert_eq!("foo ba", state.up_to_cursor());
        state.handle_event(ctrl('w'));
        assert_eq!("foo r ", state.get_value());
        assert_eq!(4, state.cursor_pos);

        // remove at start of word
        state.handle_event(ctrl('w'));
        assert_eq!("r ", state.get_value());
        assert_eq!(0, state.cursor_pos);

        // remove when buffer is empty
        state.set_value("");
        assert_eq!(0, state.cursor_pos);
        assert_consumed!(state.handle_event(ctrl('w')));
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = TextInputState::default();
        state.focus();
        state.set_value("foo bar baz");
        state.set_cursor(0);

        assert_consumed!(state.handle_event(ctrl('e')));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(11, state.cursor_pos);

        assert_consumed!(state.handle_event(ctrl('a')));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(0, state.cursor_pos);

        assert_consumed!(state.handle_event(alt('f')));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(4, state.cursor_pos);

        state.handle_event(alt('f'));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(8, state.cursor_pos);

        state.handle_event(alt('f'));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(11, state.cursor_pos);

        assert_consumed!(state.handle_event(alt('b')));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(8, state.cursor_pos);

        state.handle_event(alt('b'));
        assert_eq!("foo bar baz", state.get_value());
        assert_eq!(4, state.cursor_pos);
    }

    // helper macros + functions
    fn ctrl(c: char) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::CONTROL,
        })
    }
    fn alt(c: char) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::ALT,
        })
    }
    fn plain(c: char) -> Event {
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
        })
    }
    fn code(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
        })
    }
    fn bksp() -> Event {
        code(KeyCode::Backspace)
    }
}
