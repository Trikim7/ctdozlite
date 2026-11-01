//! Do not treat Enter used to commit an IME composition as "send message".
use eframe::egui::{Event, ImeEvent};
#[derive(Default)]
pub struct ImeState {
    composing: bool,
    activity_this_frame: bool,
}
impl ImeState {
    pub fn update(&mut self, events: &[Event]) {
        self.activity_this_frame = false;
        for event in events {
            match event {
                Event::Ime(ImeEvent::Preedit(text)) => {
                    self.composing = !text.is_empty();
                    self.activity_this_frame = true;
                }
                Event::Ime(ImeEvent::Commit(_)) => {
                    self.composing = false;
                    self.activity_this_frame = true;
                }
                Event::Ime(ImeEvent::Disabled) | Event::WindowFocused(false) => {
                    self.composing = false;
                }
                _ => {}
            }
        }
    }
    pub fn may_submit_enter(&self) -> bool {
        !self.composing && !self.activity_this_frame
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn enter_commits_vietnamese_before_it_can_send() {
        let mut state = ImeState::default();
        state.update(&[
            Event::Ime(ImeEvent::Enabled),
            Event::Ime(ImeEvent::Preedit("tiếng Việt".into())),
        ]);
        assert!(!state.may_submit_enter());
        state.update(&[]);
        assert!(!state.may_submit_enter());
        state.update(&[Event::Ime(ImeEvent::Commit("tiếng Việt".into()))]);
        assert!(!state.may_submit_enter());
        state.update(&[]);
        assert!(state.may_submit_enter());
    }
    #[test]
    fn cancellation_and_focus_loss_do_not_leave_composer_stuck() {
        let mut state = ImeState::default();
        state.update(&[Event::Ime(ImeEvent::Preedit("đ".into()))]);
        state.update(&[Event::WindowFocused(false)]);
        assert!(state.may_submit_enter());
        state.update(&[Event::Ime(ImeEvent::Preedit("".into()))]);
        assert!(!state.may_submit_enter());
        state.update(&[]);
        assert!(state.may_submit_enter());
    }
}
