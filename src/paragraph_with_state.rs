use crate::ui::paragraph::WrappedCache;
use crate::ScrollDirection;
use crossterm::event::{KeyCode, KeyEvent};

use std::sync::Arc;

pub struct ParagraphWithState {
    value: String,
    pub cache: Option<Arc<WrappedCache>>,
    pub scroll: u16,
    supports_scroll: bool,
    supports_editing: bool,
}

impl ParagraphWithState {
    pub fn new(init_value: String, supports_scroll: bool, supports_editing: bool) -> Self {
        ParagraphWithState {
            value: init_value,
            cache: None,
            scroll: 0,
            supports_scroll,
            supports_editing,
        }
    }

    pub fn reset(&mut self) {
        self.scroll = 0;
        self.value = "".to_string();
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn as_str(&self) -> &str {
        self.value.as_str()
    }

    pub fn set_value(&mut self, value: String) {
        self.value = value;
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up => self.scroll(ScrollDirection::Up),
            KeyCode::Down => self.scroll(ScrollDirection::Down),
            KeyCode::Char(_) | KeyCode::Backspace => {
                self.edit(key.code);
            }
            _ => {}
        };
    }

    fn edit(&mut self, code: KeyCode) {
        if !self.supports_editing {
            return;
        }
        match code {
            KeyCode::Char(c) => {
                self.value.push(c);
            }
            KeyCode::Backspace => {
                self.value.pop();
            }
            _ => {}
        }
    }

    fn scroll(&mut self, direction: ScrollDirection) {
        if !self.supports_scroll {
            return;
        }

        match direction {
            ScrollDirection::Up => {
                if self.scroll != 0 {
                    self.scroll -= 1;
                }
            }
            ScrollDirection::Down => {
                self.scroll += 1;
            }
        };
    }

    pub fn update(&mut self, update: (u16, Arc<WrappedCache>)) {
        self.scroll = update.0;
        self.cache = Some(update.1);
    }
}
