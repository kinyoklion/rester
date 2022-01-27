#[macro_use]
extern crate log;
extern crate simplelog;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rester::ui::paragraph::{paragraph, paragraph_color};

use log::LevelFilter;

use simplelog::{CombinedLogger, Config, WriteLogger};
use std::fs::File;

use std::str;

use std::io;
use std::sync::atomic::Ordering;

use std::thread::sleep;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use rester::app::{App, Modal, Mode};
use rester::ui::centered_rect;
use rester::ui::text_area::TextArea;
use rester::web_request_handler;
use tui::style::Modifier;
use tui::widgets::{Clear, List, ListItem};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    CombinedLogger::init(vec![WriteLogger::new(
        LevelFilter::Info,
        Config::default(),
        File::create("rester.log").unwrap(),
    )])
    .unwrap();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (sender, receiver) = mpsc::channel(10);
    let app = App::new(sender);

    web_request_handler::web_request_handler(receiver);

    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let mut needs_render = true;
    loop {
        if needs_render {
            terminal.draw(|f| ui(f, &mut app))?;
            needs_render = false;
        }

        // Poll with a timeout used a lot more CPU than expected.
        // So, for now, it just sleeps for 16ms, then checks for any stimulus.
        sleep(Duration::from_millis(16));
        loop {
            if let Ok(present) = event::poll(Duration::from_millis(0)) {
                if present {
                    let start = Instant::now();
                    if let Event::Key(key) = event::read()? {
                        if app.handle_input(key) {
                            return Ok(());
                        }
                    }

                    let duration = start.elapsed();

                    info!("Time elapsed input handling is: {:?}", duration);
                    needs_render = true;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if app.dirty.swap(false, Ordering::SeqCst) {
            needs_render = true;
        }
    }
}

fn ui<B: Backend>(rect: &mut Frame<B>, app: &mut App) {
    let start = Instant::now();
    let size = rect.size();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(size);

    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)].as_ref())
        .split(chunks[1]);

    let response_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(horizontal_chunks[1]);

    let mut header_response_paragraph = app.response_header_paragraph.lock().unwrap();

    let header_updates = paragraph(
        rect,
        response_chunks[1],
        "Response Headers",
        header_response_paragraph.as_str(),
        app.mode == Mode::ResponseHeaders,
        header_response_paragraph.scroll,
        header_response_paragraph.cache.clone(),
    );

    header_response_paragraph.update(header_updates);

    let mut response_paragraph = app.response_paragraph.lock().unwrap();

    let res = paragraph(
        rect,
        response_chunks[0],
        "Response Body",
        response_paragraph.as_str(),
        app.mode == Mode::ResponseBody,
        response_paragraph.scroll,
        response_paragraph.cache.clone(),
    );
    response_paragraph.update(res);

    let side_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Percentage(50),
            ]
            .as_ref(),
        )
        .split(horizontal_chunks[0]);

    let method_str: &'static str = app.method.into();

    paragraph(
        rect,
        side_chunks[0],
        "Method",
        method_str,
        app.mode == Mode::Method,
        0,
        None,
    );

    let params = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Params")
        .border_type(if app.mode == Mode::UrlParams {
            BorderType::Double
        } else {
            BorderType::Plain
        });
    rect.render_widget(params, side_chunks[1]);

    rect.render_stateful_widget(
        TextArea::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("Headers")
                    .border_type(if app.mode == Mode::RequestHeaders {
                        BorderType::Double
                    } else {
                        BorderType::Plain
                    }),
            )
            .active(app.mode == Mode::RequestHeaders),
        side_chunks[2],
        &mut app.headers,
    );

    rect.render_stateful_widget(
        TextArea::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::White))
                    .title("Url")
                    .border_type(if app.mode == Mode::Url {
                        BorderType::Double
                    } else {
                        BorderType::Plain
                    }),
            )
            .active(app.mode == Mode::Url),
        chunks[0],
        &mut app.url,
    );
    // paragraph_color(
    //     rect,
    //     chunks[0],
    //     "URL",
    //     app.url.as_str(),
    //     app.mode == Mode::Url,
    //     0,
    //     Color::LightCyan,
    //     None,
    // );

    let copyright = Paragraph::new("Ryan Lamb 2022")
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("Copyright")
                .border_type(BorderType::Plain),
        );

    if app.modal == Modal::Requests {
        let block = Block::default().style(Style::default().bg(Color::Blue));
        rect.render_widget(block.clone(), chunks[1]);
        rect.render_widget(block.clone(), chunks[0]);
        rect.render_widget(block, chunks[2]);

        let area = centered_rect(60, 60, size);
        rect.render_widget(Clear, area);

        let items: Vec<ListItem> = app
            .request_collection
            .requests
            .iter()
            .map(|i| ListItem::new(i.key.as_str()))
            .collect();
        let items = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Requests"))
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        rect.render_stateful_widget(items, area, &mut app.request_selection_state);
    }

    if app.modal == Modal::Save {
        let block = Block::default().style(Style::default().bg(Color::Blue));
        rect.render_widget(block.clone(), chunks[1]);
        rect.render_widget(block.clone(), chunks[0]);
        rect.render_widget(block, chunks[2]);

        let area = centered_rect(60, 20, size);
        rect.render_widget(Clear, area);
        paragraph_color(
            rect,
            area,
            "Request Name",
            app.request_name.as_str(),
            true,
            0,
            Color::Cyan,
            None,
        );
    }

    rect.render_widget(copyright, chunks[2]);
    let duration = start.elapsed();

    info!("Time elapsed rendering ui is: {:?}", duration);
}
