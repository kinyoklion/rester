use tui::buffer::Buffer;
use tui::layout::Rect;
use tui::style::Color;
use tui::widgets::Widget;

#[derive(Default)]
pub struct Cursor {
    position: [u16; 2],
    /// Scroll position x, y
    scroll: [u16; 2],
}

impl Widget for Cursor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.get_mut(
            area.x + self.position[0] - self.scroll[0],
            area.y + self.position[1] - self.scroll[1],
        )
        .set_bg(Color::Cyan);
    }
}

impl Cursor {
    pub fn position(mut self, x: u16, y: u16) -> Self {
        self.position[0] = x;
        self.position[1] = y;
        self
    }

    /// The parameters are what would normally be backward to match tui-rs.
    pub fn scroll(mut self, y: u16, x: u16) -> Self {
        self.scroll[0] = x;
        self.scroll[1] = y;
        self
    }
}
