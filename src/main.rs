use crate::Mode::Url;

use bytes::Bytes;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rester::ui::paragraph::{paragraph, paragraph_color};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str;
use std::str::{FromStr, Utf8Error};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{error::Error, io};
use strum_macros::IntoStaticStr;

use tokio::sync::{mpsc, oneshot};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame, Terminal,
};

#[derive(Copy, Clone, PartialEq, Debug)]
enum Mode {
    Url,
    Method,
    UrlParams,
    RequestHeaders,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Copy, Clone, PartialEq, IntoStaticStr, Debug)]
enum Method {
    GET,
    POST,
}

type Responder<T> = oneshot::Sender<T>;

#[derive(Debug)]
enum Response {
    Success(Bytes),
    Failure,
}

#[derive(Debug)]
enum ScrollDirection {
    Up,
    Down,
}

#[derive(Debug)]
struct Request {
    method: Method,
    url: String,
    headers: String,
    resp: Responder<Response>,
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
        let headers = self.headers.clone();
        tokio::spawn(async move {
            let (tx, rx) = oneshot::channel();
            sender
                .send(Request {
                    method,
                    url,
                    headers,
                    resp: tx,
                })
                .await
                .unwrap();
            let res = rx.await;
            match res {
                Ok(Response::Success(res)) => {
                    let mut response_bytes = response.lock().unwrap();
                    *response_bytes = Some(res);
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
            scroll_states: ScrollStates { response: 0 },
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (sender, mut receiver) = mpsc::channel(10);
    let app = App::new(sender.clone());

    tokio::spawn(async move {
        loop {
            let client = reqwest::Client::new();
            let req = receiver.recv().await;
            // println!("Request {:?}", req);
            match req {
                Some(req) => {
                    // println!("Request {:?}", req);
                    let mut header_map = HeaderMap::new();
                    let headers: Vec<&str> = req.headers.split("\r\n").collect();

                    for entry in headers {
                        if let Some((key, value)) = entry.split_once(":") {
                            if let Ok(value) = HeaderValue::from_str(value.trim()) {
                                if let Ok(key) = HeaderName::from_str(key.trim()) {
                                    header_map.append(key, value);
                                }
                            }
                        }
                    }
                    let res = client.get(req.url).headers(header_map).send().await;
                    // println!("Got {:?}", res);
                    match res {
                        Ok(res) => {
                            let bytes = res.bytes().await;
                            if let Ok(bytes) = bytes {
                                req.resp.send(Response::Success(bytes));
                            }
                        }
                        Err(_) => {
                            req.resp.send(Response::Failure);
                        }
                    };
                }
                _ => {}
            };
        }
    });

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
            }
        }
    }
}

fn ui<B: Backend>(rect: &mut Frame<B>, app: &App) {
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

    let option = app.response.lock().unwrap();
    let response_string = match &(*option) {
        None => "".to_string(),
        Some(bytes) => String::from_utf8_lossy(bytes).to_string(),
    };

    let pretty_json = jsonxf::pretty_print(response_string.as_str());

    let final_string = if let Ok(pretty_json) = pretty_json {
        pretty_json
    } else {
        response_string
    };

    paragraph(
        rect,
        response_chunks[0],
        "Response Body",
        final_string.as_str(),
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
}
