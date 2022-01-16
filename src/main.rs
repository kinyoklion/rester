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
use rester::ui::paragraph::{paragraph, paragraph_color, WrappedCache};

use log::LevelFilter;

use simplelog::{CombinedLogger, Config, WriteLogger};
use std::fs::File;
use std::rc::Rc;
use std::str;

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use rester::{web_request_handler, Method, Request, Response};
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

#[derive(Debug)]
enum ScrollDirection {
    Up,
    Down,
}

struct ScrollStates {
    response: u16,
    response_headers: u16,
}

struct CacheStates {
    response: Option<Rc<WrappedCache>>,
    response_headers: Option<Rc<WrappedCache>>,
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
    response_header_string: Arc<Mutex<Option<String>>>,
    scroll_states: ScrollStates,
    cache_states: CacheStates,
    dirty: Arc<AtomicBool>,
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
            Mode::ResponseHeaders => match direction {
                ScrollDirection::Up => {
                    if self.scroll_states.response_headers != 0 {
                        self.scroll_states.response_headers -= 1;
                    }
                }
                ScrollDirection::Down => {
                    self.scroll_states.response_headers += 1;
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

    fn handle_response_headers_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.scroll(ScrollDirection::Up),
            KeyCode::Down => self.scroll(ScrollDirection::Down),
            _ => {}
        };
    }

    fn reset(&mut self) {
        self.scroll_states.response = 0;
        self.scroll_states.response_headers = 0;
        *self.response_string.lock().unwrap() = None;
        *self.response_header_string.lock().unwrap() = None;
        *self.response.lock().unwrap() = None;
    }

    fn make_request(&mut self) {
        self.reset();
        let sender = self.sender.clone();
        let method = self.method;
        let url = self.url.clone();
        let response = self.response.clone();
        let res_string = self.response_string.clone();
        let headers = self.headers.clone();
        let dirty = self.dirty.clone();
        let response_header_string = self.response_header_string.clone();

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

            loop {
                let res = rx.recv().await;

                match res {
                    Some(Response::Headers(res)) => {
                        let header_string = jsonxf::pretty_print(format!("{:?}", res).as_str());
                        if let Ok(header_string) = header_string {
                            let mut response_header = response_header_string.lock().unwrap();
                            *response_header = Some(header_string);
                        }
                    }
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
                        dirty.store(true, Ordering::SeqCst);
                    }
                    _ => {
                        break;
                    }
                };
            }
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
            scroll_states: ScrollStates {
                response: 0,
                response_headers: 0,
            },
            cache_states: CacheStates {
                response: None,
                response_headers: None,
            },
            dirty: Arc::new(AtomicBool::new(false)),
            response_header_string: Arc::new(Mutex::new(None)),
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
                        KeyCode::Tab => {
                            app.next_mode();
                        }
                        code => match app.mode {
                            Mode::Url => app.handle_url_input(code),
                            Mode::RequestHeaders => app.handle_headers_input(code),
                            Mode::ResponseBody => app.handle_response_input(code),
                            Mode::ResponseHeaders => app.handle_response_headers_input(code),
                            _ => {}
                        },
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
