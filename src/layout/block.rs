use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, BorderType};

pub fn block(title: &str, active: bool) -> Block {
    Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title(title)
        .border_type(if active {
            BorderType::Double
        } else {
            BorderType::Plain
        })
}