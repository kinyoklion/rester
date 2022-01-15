#[macro_use]
extern crate log;
extern crate simplelog;

use crate::Mode::Url;

use bytes::Bytes;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rester::ui::paragraph::{paragraph, paragraph_color};

use log::LevelFilter;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use simplelog::{CombinedLogger, Config, WriteLogger};
use std::fs::File;
use std::str;
use std::str::{FromStr, Utf8Error};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{error::Error, io};

use tokio::sync::mpsc::Receiver;
use tokio::sync::{mpsc, oneshot};

use rester::{web_request_handler, Method, Request, Response};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

#[derive(Copy, Clone, PartialEq, Debug)]
pub(crate) enum Mode {
    Url,
    Method,
    UrlParams,
    RequestHeaders,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Debug)]
enum ScrollDirection {
    Up,
    Down,
}

struct ScrollStates {
    response: u16,
}

/// App holds the state of the application
struct App {
    url: String,
    mode: Mode,
    method: Method,
    headers: String,
    sender: mpsc::Sender<Request>,
    response: Arc<Mutex<Option<Bytes>>>,
    response_string: Arc<Mutex<Option<String>>>,
    scroll_states: ScrollStates,
}

impl App {
    fn next_mode(&mut self) {
        static MODES: [Mode; 6] = [
            Mode::Url,
            Mode::Method,
            Mode::UrlParams,
            Mode::RequestHeaders,
            Mode::ResponseHeaders,
            Mode::ResponseBody,
        ];
        let mut index = MODES.iter().position(|mode| mode == &self.mode).unwrap();
        index += 1usize;
        if index < MODES.len() {
            self.mode = MODES[index];
        } else {
            self.mode = MODES[0usize];
        }
    }

    fn handle_url_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                self.make_request();
                // app.messages.push(app.input.drain(..).collect());
            }
            KeyCode::Char(c) => {
                self.url.push(c);
            }
            KeyCode::Backspace => {
                self.url.pop();
            }
            _ => {}
        };
    }

    fn handle_headers_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                // self.make_request();
                // app.messages.push(app.input.drain(..).collect());
                self.headers.push('\r');
                self.headers.push('\n');
            }
            KeyCode::Char(c) => {
                self.headers.push(c);
            }
            KeyCode::Backspace => {
                self.headers.pop();
            }
            _ => {}
        };
    }

    fn scroll(&mut self, direction: ScrollDirection) {
        match self.mode {
            Mode::ResponseBody => match direction {
                ScrollDirection::Up => {
                    if self.scroll_states.response != 0 {
                        self.scroll_states.response -= 1;
                    }
                }
                ScrollDirection::Down => {
                    self.scroll_states.response += 1;
                }
            },
            _ => {}
        };
    }

    fn handle_response_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.scroll(ScrollDirection::Up),
            KeyCode::Down => self.scroll(ScrollDirection::Down),
            _ => {}
        };
    }

    fn make_request(&self) {
        let sender = self.sender.clone();
        let method = self.method;
        let url = self.url.clone();
        let response = self.response.clone();
        let res_string = self.response_string.clone();
        let headers = self.headers.clone();
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(10);
            sender
                .send(Request {
                    method,
                    url,
                    headers,
                    resp: tx,
                })
                .await
                .unwrap();
            let res = rx.recv().await;
            match res {
                Some(Response::Body(res)) => {
                    let mut response_bytes = response.lock().unwrap();

                    let mut response_string = res_string.lock().unwrap();

                    let decoded_string = String::from_utf8_lossy(&res);
                    let pretty_json = jsonxf::pretty_print(decoded_string.to_string().as_str());

                    let final_string = if let Ok(pretty_json) = pretty_json {
                        pretty_json
                    } else {
                        decoded_string.to_string()
                    };

                    *response_bytes = Some(res);
                    *response_string = Some(final_string);
                }
                _ => {}
            };
        });
    }
}

impl App {
    fn new(sender: mpsc::Sender<Request>) -> Self {
        App {
            url: String::new(),
            headers: String::new(),
            mode: Url,
            method: Method::GET,
            sender,
            response: Arc::new(Mutex::new(None)),
            response_string: Arc::new(Mutex::new(None)),
            scroll_states: ScrollStates { response: 0 },
        }
    }
}

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

    let (sender, mut receiver) = mpsc::channel(10);
    let app = App::new(sender.clone());

    web_request_handler::WebRequestHandler(receiver);

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

    return Ok(());
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Ok(present) = event::poll(Duration::from_millis(16)) {
            if present {
                let start = Instant::now();
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Esc => {
                            return Ok(());
                            // app.messages.push(app.input.drain(..).collect());
                        }
                        KeyCode::Tab => {
                            app.next_mode();
                        }
                        code => match app.mode {
                            Mode::Url => app.handle_url_input(code),
                            Mode::RequestHeaders => app.handle_headers_input(code),
                            Mode::ResponseBody => app.handle_response_input(code),
                            _ => {}
                        },
                    }
                }

                let duration = start.elapsed();

                info!("Time elapsed input handling is: {:?}", duration);
            }
        }
    }
}

fn ui<B: Backend>(rect: &mut Frame<B>, app: &App) {
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

    let response_headers = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Response Header")
        .border_type(if app.mode == Mode::ResponseHeaders {
            BorderType::Double
        } else {
            BorderType::Plain
        });
    rect.render_widget(response_headers, response_chunks[1]);

    let option = app.response_string.lock().unwrap();

    let response_string = match &(*option) {
        None => "",
        Some(string) => string.as_str(),
    };

    paragraph(
        rect,
        response_chunks[0],
        "Response Body",
        response_string,
        app.mode == Mode::ResponseBody,
        app.scroll_states.response,
    );

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
    );

    paragraph_color(
        rect,
        chunks[0],
        "URL",
        app.url.as_ref(),
        app.mode == Mode::Url,
        0,
        Color::LightCyan,
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

    // info!("Time elapsed in expensive_function() is: {:?}", duration);
}
