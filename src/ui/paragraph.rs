use tui::backend::Backend;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use tui::Frame;

pub fn paragraph<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
) {
    paragraph_color(app_rect, rect, title, text, active, scroll, Color::White);
}

pub fn paragraph_color<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
    color: Color,
) {
    let response_body = Paragraph::new(text)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::LightCyan))
        .style(Style::default().fg(color))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
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
