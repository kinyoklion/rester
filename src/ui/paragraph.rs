use tui::backend::Backend;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};
use tui::Frame;

fn count_newlines(s: &str) -> u16 {
    s.as_bytes().iter().filter(|&&c| c == b'\n').count() as u16
}

pub fn paragraph<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
) -> u16 {
    paragraph_color(app_rect, rect, title, text, active, scroll, Color::White)
}

pub fn paragraph_color<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
    color: Color,
) -> u16 {
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title(title)
        .border_type(if active {
            BorderType::Double
        } else {
            BorderType::Plain
        });
    let inner_rect = block.inner(rect);
    info!("Width {:?}", inner_rect.width);
    let wrapped = textwrap::fill(text, inner_rect.width as usize);
    let lines = count_newlines(wrapped.as_str());

    let height_adjusted_lines = if lines >= inner_rect.height {
        (lines - inner_rect.height) + 1
    } else {
        0
    };

    let capped_scroll = if scroll > height_adjusted_lines {
        height_adjusted_lines
    } else {
        scroll
    };

    let response_body = Paragraph::new(wrapped.as_str())
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::LightCyan))
        .style(Style::default().fg(color))
        // .wrap(Wrap { trim: false })
        .scroll((capped_scroll, 0))
        .block(block);
    app_rect.render_widget(response_body, rect);
    capped_scroll
}
