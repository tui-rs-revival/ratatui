use std::{borrow::Borrow, collections::HashSet};

use super::{Block, ListItem, StatefulWidget, Widget};
use crate::{
    layout::{Corner, Rect},
    style::Style,
};

#[derive(Clone, Debug)]
pub struct MultiListState {
    selected: HashSet<usize>,
    highlighted: Option<usize>,
    offset: usize,
}

impl Default for MultiListState {
    fn default() -> Self {
        Self {
            selected: HashSet::new(),
            highlighted: None,
            offset: 0,
        }
    }
}

impl MultiListState {
    pub fn select(&mut self, i: usize) {
        self.selected.insert(i);
    }

    pub fn deselect(&mut self, i: usize) {
        self.selected.remove(&i);
    }

    pub fn toggle_selection(&mut self, i: usize) {
        if self.selected.contains(&i) {
            self.selected.remove(&i);
        } else {
            self.selected.insert(i);
        }
    }

    pub fn highlight(&mut self, i: Option<usize>) {
        self.highlighted = i;
    }

    pub fn get_highlight(&mut self) -> Option<usize> {
        self.highlighted
    }

    pub fn get_selections(&self) -> &HashSet<usize> {
        self.selected.borrow()
    }
}

#[derive(Debug, Clone)]
pub struct MutliList<'a> {
    block: Option<Block<'a>>,
    items: Vec<ListItem<'a>>,
    style: Style,
    start_corner: Corner,
    selected_style: Style,
    highlight_style: Style,
    highlight_symbol: Option<&'a str>,
}

impl<'a> MutliList<'a> {
    pub fn new<T>(items: T) -> Self
    where
        T: Into<Vec<ListItem<'a>>>,
    {
        Self {
            block: None,
            style: Style::default(),
            items: items.into(),
            start_corner: Corner::TopLeft,
            selected_style: Style::default(),
            highlight_style: Style::default(),
            highlight_symbol: None,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn highlight_symbol(mut self, highlight_symbol: &'a str) -> Self {
        self.highlight_symbol = Some(highlight_symbol);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn start_corner(mut self, corner: Corner) -> Self {
        self.start_corner = corner;
        self
    }

    pub fn selected_style(mut self, selected_style: Style) -> Self {
        self.selected_style = selected_style;
        self
    }

    fn get_items_bounds(
        &self,
        highlighted: Option<usize>,
        offset: usize,
        max_height: usize,
    ) -> (usize, usize) {
        let offset = offset.min(self.items.len().saturating_sub(1));
        let mut start = offset;
        let mut end = offset;
        let mut height = 0;
        for item in self.items.iter().skip(offset) {
            if height + item.height() > max_height {
                break;
            }
            height += item.height();
            end += 1;
        }

        let selected = highlighted.unwrap_or(0).min(self.items.len() - 1);
        while selected >= end {
            height = height.saturating_add(self.items[end].height());
            end += 1;
            while height > max_height {
                height = height.saturating_sub(self.items[start].height());
                start += 1;
            }
        }
        while selected < start {
            start -= 1;
            height = height.saturating_add(self.items[start].height());
            while height > max_height {
                end -= 1;
                height = height.saturating_sub(self.items[end].height());
            }
        }
        (start, end)
    }
}

impl<'a> Widget for MutliList<'a> {
    fn render(self, area: crate::layout::Rect, buf: &mut crate::buffer::Buffer) {
        let mut state = MultiListState::default();
        StatefulWidget::render(self, area, buf, &mut state);
    }
}

impl<'a> StatefulWidget for MutliList<'a> {
    type State = MultiListState;

    fn render(
        mut self,
        area: crate::layout::Rect,
        buf: &mut crate::buffer::Buffer,
        state: &mut Self::State,
    ) {
        buf.set_style(area, self.style);
        let list_area = match self.block.take() {
            Some(b) => {
                let inner_area = b.inner(area);
                b.render(area, buf);
                inner_area
            }
            None => area,
        };

        if list_area.width < 1 || list_area.height < 1 {
            return;
        }

        if self.items.is_empty() {
            return;
        }
        let list_height = list_area.height as usize;

        let (start, end) = self.get_items_bounds(state.highlighted, state.offset, list_height);
        state.offset = start;

        let highlight_symbol = self
            .highlight_symbol
            .map(|s| String::from(s))
            .unwrap_or("".into());

        let mut current_height = 0;

        for (i, item) in self
            .items
            .iter_mut()
            .enumerate()
            .skip(state.offset)
            .take(end - start)
        {
            let (x, y) = match self.start_corner {
                Corner::BottomLeft => {
                    current_height += item.height() as u16;
                    (list_area.left(), list_area.bottom() - current_height)
                }
                _ => {
                    let pos = (list_area.left(), list_area.top() + current_height);
                    current_height += item.height() as u16;
                    pos
                }
            };
            let area = Rect {
                x,
                y,
                width: list_area.width,
                height: item.height() as u16,
            };
            let item_style = self.style.patch(item.style);
            buf.set_style(area, item_style);

            let is_selected = state.selected.contains(&i);

            let is_highlighted = state.highlighted.map(|h| h == i).unwrap_or(false);

            let elem_x = if is_highlighted {
                let (x, _) = buf.set_stringn(
                    x,
                    y,
                    highlight_symbol.clone(),
                    list_area.width as usize,
                    item_style,
                );
                x
            } else {
                x
            };

            let max_element_width = (list_area.width - (elem_x - x)) as usize;
            for (j, line) in item.content.lines.iter().enumerate() {
                buf.set_spans(elem_x, y + j as u16, line, max_element_width as u16);
            }
            let mut style = if is_selected {
                self.selected_style
            } else {
                self.style
            };

            if is_highlighted {
                style = style.patch(self.highlight_style);
            }

            buf.set_style(area, style);
        }
    }
}
