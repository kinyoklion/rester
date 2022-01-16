#[macro_use]
extern crate log;
extern crate simplelog;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
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

use rester::app::{App, Mode};
use rester::web_request_handler;
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
        if let Ok(present) = event::poll(Duration::from_millis(0)) {
            if present {
                let start = Instant::now();
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            return Ok(());
                            // app.messages.push(app.input.drain(..).collect());
                        }
                        _ => app.handle_input(key),
                    }
                }

                let duration = start.elapsed();

                info!("Time elapsed input handling is: {:?}", duration);
                needs_render = true;
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

    let header_option = app.response_header_string.lock().unwrap();
    let headers_response_string = match &(*header_option) {
        None => "",
        Some(string) => string.as_str(),
    };

    let header_updates = paragraph(
        rect,
        response_chunks[1],
        "Response Headers",
        headers_response_string,
        app.mode == Mode::ResponseHeaders,
        app.scroll_states.response_headers,
        app.cache_states.response_headers.clone(),
    );

    app.scroll_states.response_headers = header_updates.0;
    app.cache_states.response_headers = Some(header_updates.1);

    let option = app.response_string.lock().unwrap();

    let response_string = match &(*option) {
        None => "",
        Some(string) => string.as_str(),
    };

    let response_updates = paragraph(
        rect,
        response_chunks[0],
        "Response Body",
        response_string,
        app.mode == Mode::ResponseBody,
        app.scroll_states.response,
        app.cache_states.response.clone(),
    );
    app.scroll_states.response = response_updates.0;
    app.cache_states.response = Some(response_updates.1);

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

    paragraph_color(
        rect,
        side_chunks[2],
        "Headers",
        app.headers.as_ref(),
        app.mode == Mode::RequestHeaders,
        0,
        Color::LightCyan,
        None,
    );

    paragraph_color(
        rect,
        chunks[0],
        "URL",
        app.url.as_ref(),
        app.mode == Mode::Url,
        0,
        Color::LightCyan,
        None,
    );

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

    rect.render_widget(copyright, chunks[2]);
    let duration = start.elapsed();

    info!("Time elapsed rendering ui is: {:?}", duration);
}
