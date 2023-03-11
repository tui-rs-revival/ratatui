use crossterm::event::Event;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InteractionOutcome {
    Consumed,
    Bubble,
}

impl InteractionOutcome {
    pub fn is_consumed(&self) -> bool {
        matches!(self, InteractionOutcome::Consumed)
    }
    pub fn is_bubble(&self) -> bool {
        matches!(self, InteractionOutcome::Bubble)
    }
}

pub trait InteractiveWidgetState {
    fn handle_event(&mut self, _event: Event) -> InteractionOutcome {
        InteractionOutcome::Bubble
    }
    fn is_focused(&self) -> bool;
    fn focus(&mut self);
    fn unfocus(&mut self);
}
