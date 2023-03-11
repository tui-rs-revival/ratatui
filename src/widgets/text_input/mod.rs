use std::borrow::Cow;

use crate::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Text},
    widgets::Block,
};

use super::{InteractiveWidget, Paragraph};

#[cfg(feature = "crossterm")]
mod crossterm_interactive;

#[derive(Debug, Clone)]
pub struct TextInput<'a> {
    // Block to draw the text input inside (convenience function) - default: None
    optional_block: Option<Block<'a>>,
    // Placeholder text - what's shown if the state value is "" - default: None
    placeholder: Option<Text<'a>>,
    // Render as a read-only input - that is, it will not be focused - default: false
    is_read_only: bool,
    // Style to render the widget when focused - default: Bold style
    focused_style: Style,
    // Style to apply to displayed text - overriden by focused_style when focused
    text_style: Style,
}

impl<'a> TextInput<'a> {
    pub fn new() -> TextInput<'a> {
        Default::default()
    }

    pub fn block(mut self, block: Block<'a>) -> TextInput<'a> {
        self.optional_block = Some(block);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> TextInput<'a> {
        self.is_read_only = read_only;
        self
    }

    pub fn placeholder_text<T>(mut self, placeholder_text: T) -> TextInput<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        self.placeholder = Some(
            Span::styled(
                placeholder_text,
                Style::default()
                    .fg(Color::Black)
                    .add_modifier(Modifier::ITALIC),
            )
            .into(),
        );
        self
    }

    pub fn placeholder(mut self, placeholder: Text<'a>) -> TextInput<'a> {
        self.placeholder = Some(placeholder);
        self
    }

    pub fn focused_style(mut self, style: Style) -> TextInput<'a> {
        self.focused_style = style;
        self
    }

    pub fn text_style(mut self, style: Style) -> TextInput<'a> {
        self.text_style = style;
        self
    }
}

impl<'a> Default for TextInput<'a> {
    fn default() -> Self {
        Self {
            optional_block: Default::default(),
            placeholder: Default::default(),
            is_read_only: false,
            focused_style: Style::default().add_modifier(Modifier::BOLD),
            text_style: Default::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TextInputState {
    // Underlying value of the text input field
    value: String,
    // Position in the text input to insert / remove text from
    cursor_pos: usize,
    // Is the input focused?
    is_focused: bool,
    // Can the input take focus?
    can_take_focus: bool,
}

impl TextInputState {
    pub fn with_value(value: &str) -> TextInputState {
        TextInputState {
            value: value.to_string(),
            cursor_pos: value.len(),
            ..Default::default()
        }
    }

    pub fn can_take_focus(&mut self, can_take_focus: bool) {
        self.can_take_focus = can_take_focus;
        if !can_take_focus {
            self.unfocus();
        }
    }
    pub fn is_focused(&self) -> bool {
        self.can_take_focus && self.is_focused
    }
    pub fn focus(&mut self) {
        if self.can_take_focus {
            self.is_focused = true;
        }
    }
    pub fn unfocus(&mut self) {
        self.is_focused = false;
    }
    pub fn set_value(&mut self, val: &str) {
        self.value = val.to_string();
        self.cursor_pos = std::cmp::min(self.cursor_pos, self.value.len());
    }
    pub fn set_cursor(&mut self, pos: usize) {
        self.cursor_pos = pos;
    }
    pub fn get_value(&self) -> &String {
        &self.value
    }
}

impl Default for TextInputState {
    fn default() -> Self {
        Self {
            value: Default::default(),
            is_focused: false,
            cursor_pos: 0,
            can_take_focus: true,
        }
    }
}

impl<'a> InteractiveWidget for TextInput<'a> {
    type State = TextInputState;

    fn render<'b, B: crate::backend::Backend + 'b>(
        mut self,
        area: Rect,
        frame: &mut crate::Frame<'b, B>,
        state: &Self::State,
    ) {
        let is_focused = !self.is_read_only && state.is_focused;

        let area = if let Some(block) = self.optional_block.take() {
            let block = if is_focused {
                block.style(self.focused_style)
            } else {
                block
            };

            let inner = block.inner(area);
            frame.render_widget(block, area);
            inner
        } else {
            area
        };

        let contents = if state.get_value().is_empty() {
            match self.placeholder {
                Some(placeholder) => placeholder,
                None => "".into(),
            }
        } else {
            let value = state.get_value();
            if is_focused {
                Span::styled(value, self.focused_style).into()
            } else {
                Span::styled(value, self.text_style).into()
            }
        };

        let paragraph = Paragraph::new(contents);

        frame.render_widget(paragraph, area);
        if is_focused {
            frame.set_cursor(area.x + (state.cursor_pos as u16), area.y);
        }
    }

    fn render_mut<'b, B: crate::backend::Backend + 'b>(
        self,
        area: Rect,
        frame: &mut crate::Frame<'b, B>,
        state: &mut Self::State,
    ) {
        self.render(area, frame, state);
    }
}
