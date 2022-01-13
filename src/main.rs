use crate::Mode::Url;

use bytes::Bytes;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rester::ui::paragraph::{paragraph, paragraph_color};

use std::str;

use std::sync::{Arc, Mutex};
use std::{io};
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
struct Request {
    method: Method,
    url: String,
    resp: Responder<Response>,
}

/// App holds the state of the application
struct App {
    url: String,
    mode: Mode,
    method: Method,
    sender: mpsc::Sender<Request>,
    response: Arc<Mutex<Option<Bytes>>>,
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

    fn make_request(&self) {
        let sender = self.sender.clone();
        let method = self.method;
        let url = self.url.clone();
        let response = self.response.clone();
        tokio::spawn(async move {
            let (tx, rx) = oneshot::channel();
            sender
                .send(Request {
                    method,
                    url,
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
            mode: Url,
            method: Method::GET,
            sender,
            response: Arc::new(Mutex::new(None)),
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
                    let res = client.get(req.url).send().await;
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
                    _ => {}
                },
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
        Some(bytes) => {
            format!("{:?}", bytes)
            // println!("Bytes {:?}", bytes);
            // match str::from_utf8(bytes) {
            //     Ok(str) => str,
            //     Err(_) => "wtf",
            // }
        }
    };

    paragraph(
        rect,
        response_chunks[0],
        "Response Body",
        response_string.as_str(),
        app.mode == Mode::ResponseBody,
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

    let headers = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White))
        .title("Headers")
        .border_type(if app.mode == Mode::RequestHeaders {
            BorderType::Double
        } else {
            BorderType::Plain
        });
    rect.render_widget(headers, side_chunks[2]);

    paragraph_color(
        rect,
        chunks[0],
        "URL",
        app.url.as_ref(),
        app.mode == Mode::Url,
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
