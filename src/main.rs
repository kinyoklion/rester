#[macro_use]
extern crate log;
extern crate simplelog;

use crossterm::{
    event::{self, DisableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use log::LevelFilter;
use rester::app::{App, Modal, Mode, View};
use rester::key_bind::get_help;
use rester::layout::block::block;
use rester::ui::centered_rect;
use rester::ui::paragraph::{paragraph, paragraph_color};
use rester::ui::text_area::TextArea;
use rester::{web_request_handler, Operation};
use simplelog::{CombinedLogger, Config, WriteLogger};
use std::fs::File;
use std::io;
use std::str;
use std::sync::atomic::Ordering;
use std::thread::sleep;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
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
    execute!(stdout, EnterAlternateScreen, DisableMouseCapture)?;
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

    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Length(11), Constraint::Min(11)].as_ref())
        .split(chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(75), Constraint::Percentage(25)].as_ref())
        .split(chunks[1]);

    if app.view == View::Response {
        let mut header_response_paragraph = app.response_header_paragraph.lock().unwrap();
        let status = app.status.load(Ordering::SeqCst);
        let status_string = if status != 0 {
            format!("Response Headers (Status {:})", status)
        } else {
            "Response Headers".to_string()
        };

        let header_updates = paragraph(
            rect,
            main_chunks[1],
            get_help(
                status_string.as_str(),
                Operation::GotoResponseHeaders,
                &app.key_binds,
            )
            .as_str(),
            header_response_paragraph.as_str(),
            app.mode == Mode::ResponseHeaders,
            header_response_paragraph.scroll,
            header_response_paragraph.cache.clone(),
        );

        header_response_paragraph.update(header_updates);

        let mut response_paragraph = app.response_paragraph.lock().unwrap();

        let res = paragraph(
            rect,
            main_chunks[0],
            get_help("Response Body", Operation::GotoResponseBody, &app.key_binds).as_str(),
            response_paragraph.as_str(),
            app.mode == Mode::ResponseBody,
            response_paragraph.scroll,
            response_paragraph.cache.clone(),
        );
        response_paragraph.update(res);
    }

    if app.view == View::Request {
        rect.render_stateful_widget(
            TextArea::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title(get_help(
                            "Request Body",
                            Operation::GotoRequestBody,
                            &app.key_binds,
                        ))
                        .border_type(if app.mode == Mode::RequestBody {
                            BorderType::Double
                        } else {
                            BorderType::Plain
                        }),
                )
                .active(app.mode == Mode::RequestBody),
            main_chunks[0],
            &mut app.body,
        );

        rect.render_stateful_widget(
            TextArea::default()
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::White))
                        .title(get_help(
                            "Request Headers",
                            Operation::GotoRequestHeaders,
                            &app.key_binds,
                        ))
                        .border_type(if app.mode == Mode::RequestHeaders {
                            BorderType::Double
                        } else {
                            BorderType::Plain
                        }),
                )
                .active(app.mode == Mode::RequestHeaders),
            main_chunks[1],
            &mut app.headers,
        );
    }

    let method_str: &'static str = app.method.into();

    paragraph(
        rect,
        header_chunks[0],
        get_help("Method", Operation::NextMethod, &app.key_binds).as_str(),
        method_str,
        app.mode == Mode::Method,
        0,
        None,
    );

    rect.render_stateful_widget(
        TextArea::default()
            .block(block(
                get_help("Url", Operation::GotoUrl, &app.key_binds).as_str(),
                app.mode == Mode::Url,
            ))
            .active(app.mode == Mode::Url),
        header_chunks[1],
        &mut app.url,
    );

    let help_string = format!(
        "{:} {:} {:} {:} {:} {:}",
        get_help("Req", Operation::GotoResponseView, &app.key_binds),
        get_help("Res", Operation::GotoRequestView, &app.key_binds),
        get_help("Load", Operation::LoadRequest, &app.key_binds),
        get_help("Save", Operation::SaveRequest, &app.key_binds),
        get_help("Quit", Operation::Quit, &app.key_binds),
        if app.mode != Mode::Url {
            get_help("Send", Operation::SendRequest, &app.key_binds)
        } else {
            "Send ⏎".to_string()
        }
    );

    let status_help = Paragraph::new(help_string.as_str())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(block("Help", false));

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
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Black),
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

    rect.render_widget(status_help, chunks[2]);
    let duration = start.elapsed();

    info!("Time elapsed rendering ui is: {:?}", duration);
}
