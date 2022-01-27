use std::sync::Arc;

use crate::ui::count_newlines;
use tui::backend::Backend;
use tui::layout::{Alignment, Rect};
use tui::style::{Color, Style};
use tui::widgets::{Paragraph};
use tui::Frame;
use crate::layout::block::block;

pub struct WrappedCache {
    id: usize,
    width: u16,
    wrapped: String,
    lines: u16,
}

impl WrappedCache {
    pub fn get_lines(&self) -> u16 {
        self.lines
    }
}

pub fn paragraph<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
    cache: Option<Arc<WrappedCache>>,
) -> (u16, Arc<WrappedCache>) {
    paragraph_color(
        app_rect,
        rect,
        title,
        text,
        active,
        scroll,
        Color::White,
        cache,
    )
}

pub fn paragraph_color<B: Backend>(
    app_rect: &mut Frame<B>,
    rect: Rect,
    title: &str,
    text: &str,
    active: bool,
    scroll: u16,
    color: Color,
    cache: Option<Arc<WrappedCache>>,
) -> (u16, Arc<WrappedCache>) {
    let block = block(title, active);
    let inner_rect = block.inner(rect);

    let cur_cache = match cache {
        None => make_cache(text, inner_rect),
        Some(cache) => {
            if cache.id != text.as_ptr() as *const _ as usize || cache.width != inner_rect.width {
                make_cache(text, inner_rect)
            } else {
                cache
            }
        }
    };

    let height_adjusted_lines = if cur_cache.lines >= inner_rect.height {
        (cur_cache.lines - inner_rect.height) + 1
    } else {
        0
    };

    let capped_scroll = if scroll > height_adjusted_lines {
        height_adjusted_lines
    } else {
        scroll
    };

    let response_body = Paragraph::new(cur_cache.wrapped.as_str())
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::LightCyan))
        .style(Style::default().fg(color))
        .scroll((capped_scroll, 0))
        .block(block);
    app_rect.render_widget(response_body, rect);
    (capped_scroll, cur_cache)
}

fn make_cache(text: &str, inner_rect: Rect) -> Arc<WrappedCache> {
    let wrapped = textwrap::fill(text, inner_rect.width as usize);
    let lines = count_newlines(wrapped.as_str());

    let cache = WrappedCache {
        id: text.as_ptr() as *const _ as usize,
        width: inner_rect.width,
        wrapped,
        lines,
    };
    Arc::new(cache)
}
