use crate::persistence::RequestCollection;
use crate::ui::paragraph::WrappedCache;
use crate::{Method, Request, Response};
use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use reqwest::RequestBuilder;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Url,
    Method,
    UrlParams,
    RequestHeaders,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Modal {
    Save,
    None,
}

#[derive(Debug)]
pub enum ScrollDirection {
    Up,
    Down,
}

pub struct ScrollStates {
    pub response: u16,
    pub response_headers: u16,
}

pub struct CacheStates {
    pub response: Option<Rc<WrappedCache>>,
    pub response_headers: Option<Rc<WrappedCache>>,
}

/// App holds the state of the application
pub struct App {
    pub url: String,
    pub mode: Mode,
    pub method: Method,
    pub headers: String,
    pub sender: mpsc::Sender<Request>,
    pub response: Arc<Mutex<Option<Bytes>>>,
    pub response_string: Arc<Mutex<Option<String>>>,
    pub response_header_string: Arc<Mutex<Option<String>>>,
    pub scroll_states: ScrollStates,
    pub cache_states: CacheStates,
    pub dirty: Arc<AtomicBool>,
    pub modal: Modal,
    pub request_name: String,
    request_collection: RequestCollection,
}

impl App {
    pub fn new(sender: mpsc::Sender<Request>) -> Self {
        App {
            url: String::new(),
            headers: String::new(),
            mode: Mode::Url,
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
            modal: Modal::None,
            request_name: "".to_string(),
            request_collection: RequestCollection::new(),
        }
    }
}

impl App {
    pub fn next_mode(&mut self, previous: bool) {
        static MODES: [Mode; 6] = [
            Mode::Url,
            Mode::Method,
            Mode::UrlParams,
            Mode::RequestHeaders,
            Mode::ResponseHeaders,
            Mode::ResponseBody,
        ];
        let mut index = MODES.iter().position(|mode| mode == &self.mode).unwrap();
        if previous {
            if index > 0 {
                index -= 1usize
            } else {
                index = MODES.len() - 1;
            }
        } else {
            index += 1usize;
        }

        if index < MODES.len() {
            self.mode = MODES[index];
        } else {
            self.mode = MODES[0usize];
        }
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char(c) => match c.to_ascii_lowercase() {
                    's' => {
                        if self.modal == Modal::None {
                            self.modal = Modal::Save;
                        }
                    }
                    _ => {}
                },
                _ => {}
            };
            return false;
        }
        match key.code {
            KeyCode::Esc => {
                return if self.modal == Modal::None {
                    true
                } else {
                    self.modal = Modal::None;
                    false
                }
            }
            KeyCode::Tab => {
                self.next_mode(false);
            }
            KeyCode::BackTab => {
                self.next_mode(true);
            }
            code => match self.modal {
                Modal::Save => self.handle_save_input(code),
                Modal::None => match self.mode {
                    Mode::Url => self.handle_url_input(code),
                    Mode::RequestHeaders => self.handle_headers_input(code),
                    Mode::ResponseBody => self.handle_response_input(code),
                    Mode::ResponseHeaders => self.handle_response_headers_input(code),
                    _ => {}
                },
            },
        }
        false
    }

    fn save_request(&mut self) {
        if self.url.is_empty() || self.request_name.is_empty() {
            return;
        }
        let mut builder = crate::persistence::RequestBuilder::new(self.request_name.as_str());
        builder.url(self.url.as_str());
        builder.method(self.method);
        builder.headers(self.headers.as_str());
        self.request_collection.add_request(builder.build());
        self.request_collection.save();
        // TODO: Need to implement some error handling here.
        self.modal = Modal::None;
    }

    fn handle_save_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => self.save_request(),
            KeyCode::Char(c) => {
                self.request_name.push(c);
            }
            KeyCode::Backspace => {
                self.request_name.pop();
            }
            _ => {}
        };
    }

    fn handle_url_input(&mut self, code: KeyCode) {
        match code {
            KeyCode::Enter => {
                self.make_request();
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

    pub fn make_request(&mut self) {
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
