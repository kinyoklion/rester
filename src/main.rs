use crate::Mode::Url;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use strum_macros::IntoStaticStr;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};

#[derive(Copy, Clone, PartialEq)]
enum Mode {
    Url,
    Method,
    UrlParams,
    RequestHeaders,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Copy, Clone, PartialEq, IntoStaticStr)]
enum Method {
    GET,
    POST,
}

/// App holds the state of the application
struct App {
    url: String,
    mode: Mode,
    method: Method,
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
}

impl Default for App {
    fn default() -> App {
        App {
            url: String::new(),
            mode: Url,
            method: Method::GET,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::default();
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
    let method = Paragraph::new(method_str)
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .title("Params")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(if app.mode == Mode::Method {
                    BorderType::Double
                } else {
                    BorderType::Plain
                }),
        );
    rect.render_widget(method, side_chunks[0]);

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

    let url = Paragraph::new(app.url.as_ref())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .title("URL")
                .border_type(if app.mode == Mode::Url {
                    BorderType::Double
                } else {
                    BorderType::Plain
                }),
        );

    let copyright = Paragraph::new("Ryan Lamb 2020")
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
    rect.render_widget(url, chunks[0]);
}
