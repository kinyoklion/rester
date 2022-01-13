use tui::backend::Backend;
use tui::Frame;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, BorderType, Paragraph, Wrap};

pub fn paragraph<B: Backend>(app_rect: &mut Frame<B>, rect: Rect, title: &str, text: &str, active: bool) {
    paragraph_color(app_rect, rect, title, text, active, Color::White);
}

pub fn paragraph_color<B: Backend>(app_rect: &mut Frame<B>, rect: Rect, title: &str, text: &str, active: bool, color: Color) {
    let response_body = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::LightCyan))
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title(title)
                .border_type(if active {
                    BorderType::Double
                } else {
                    BorderType::Plain
                }),
        );
    app_rect.render_widget(response_body, rect);
}