use crate::paragraph_with_state::ParagraphWithState;
use crate::persistence::RequestCollection;

use crate::{Method, Request, Response};
use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::ui::text_area::{EditCommand, EditState};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tui::widgets::ListState;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum View {
    Request,
    Response,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Mode {
    Url,
    Method,
    RequestHeaders,
    RequestBody,
    ResponseHeaders,
    ResponseBody,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Modal {
    Save,
    Requests,
    None,
}

/// App holds the state of the application
pub struct App {
    pub url: EditState,
    pub mode: Mode,
    pub method: Method,
    pub headers: EditState,
    pub body: EditState,
    pub sender: mpsc::Sender<Request>,
    pub response: Arc<Mutex<Option<Bytes>>>,
    pub response_paragraph: Arc<Mutex<ParagraphWithState>>,
    pub response_header_paragraph: Arc<Mutex<ParagraphWithState>>,
    pub dirty: Arc<AtomicBool>,
    pub modal: Modal,
    pub view: View,
    pub request_name: String,
    pub request_collection: RequestCollection,
    pub request_selection_state: ListState,
}

impl App {
    pub fn new(sender: mpsc::Sender<Request>) -> Self {
        App {
            url: EditState::new(""),
            headers: EditState::new(""),
            body: EditState::new(""),
            mode: Mode::Url,
            method: Method::GET,
            sender,
            response: Arc::new(Mutex::new(None)),
            response_paragraph: Arc::new(Mutex::new(ParagraphWithState::new(
                "".to_string(),
                true,
                false,
            ))),
            dirty: Arc::new(AtomicBool::new(false)),
            response_header_paragraph: Arc::new(Mutex::new(ParagraphWithState::new(
                "".to_string(),
                true,
                false,
            ))),
            modal: Modal::None,
            request_name: "".to_string(),
            request_collection: RequestCollection::load(),
            request_selection_state: ListState::default(),
            view: View::Request,
        }
    }
}

impl App {
    fn next_method(&mut self) {
        static METHODS: [Method; 5] = [
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::PATCH,
        ];

        let mut index = METHODS
            .iter()
            .position(|method| method == &self.method)
            .unwrap_or(0usize);

        index += 1;

        if index < METHODS.len() {
            self.method = METHODS[index];
        } else {
            self.method = METHODS[0usize];
        }
    }

    pub fn next_mode(&mut self, previous: bool) {
        static REQUEST_MODES: [Mode; 3] = [Mode::Url, Mode::RequestBody, Mode::RequestHeaders];
        static RESPONSE_MODES: [Mode; 3] = [Mode::Url, Mode::ResponseBody, Mode::ResponseHeaders];
        let modes = match self.view {
            View::Request => &REQUEST_MODES,
            View::Response => &RESPONSE_MODES,
        };
        let mut index = modes
            .iter()
            .position(|mode| mode == &self.mode)
            .unwrap_or(0usize);
        if previous {
            if index > 0 {
                index -= 1usize
            } else {
                index = modes.len() - 1;
            }
        } else {
            index += 1usize;
        }

        if index < modes.len() {
            self.mode = modes[index];
        } else {
            self.mode = modes[0usize];
        }
    }

    fn list_next(size: usize, current: usize) -> usize {
        if current >= size - 1 {
            0
        } else {
            current + 1
        }
    }

    fn list_previous(size: usize, current: usize) -> usize {
        if current == 0 {
            size - 1
        } else {
            current - 1
        }
    }

    fn set_view(&mut self, view: View) {
        if self.view != view {
            self.view = view;
            self.mode = Mode::Url
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
                    'r' => {
                        if self.modal == Modal::None {
                            self.modal = Modal::Requests;
                            self.request_selection_state.select(Some(0));
                        }
                    }
                    'p' => self.next_method(),
                    'a' => self.set_view(View::Request),
                    'q' => self.set_view(View::Response),
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
                return false;
            }
            KeyCode::BackTab => {
                self.next_mode(true);
                return false;
            }
            _ => {}
        }
        match self.modal {
            Modal::Save => self.handle_save_input(key),
            Modal::Requests => self.handle_request_input(key),
            Modal::None => match self.mode {
                Mode::Url => self.handle_url_input(key),
                Mode::RequestHeaders => self.handle_request_headers_input(key),
                Mode::RequestBody => self.handle_request_body_input(key),
                Mode::ResponseBody => self.response_paragraph.lock().unwrap().handle_input(key),
                Mode::ResponseHeaders => self
                    .response_header_paragraph
                    .lock()
                    .unwrap()
                    .handle_input(key),
                _ => {}
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

    fn handle_save_input(&mut self, key: KeyEvent) {
        match key.code {
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

    fn handle_request_input(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => {
                let index = self.request_selection_state.selected().unwrap_or(0);

                self.reset();
                let request = &self.request_collection.requests[index];

                self.url.set_value(request.url.clone());
                self.method = request.method;
                self.request_name = request.key.clone();
                self.headers.set_value(request.headers_to_string());

                self.modal = Modal::None;
            }
            KeyCode::Up => self
                .request_selection_state
                .select(Some(Self::list_previous(
                    self.request_collection.requests.len(),
                    self.request_selection_state.selected().unwrap_or(0),
                ))),
            KeyCode::Down => self.request_selection_state.select(Some(Self::list_next(
                self.request_collection.requests.len(),
                self.request_selection_state.selected().unwrap_or(0),
            ))),
            KeyCode::Delete => {
                if let Some(index) = self.request_selection_state.selected() {
                    self.request_collection.remove_request(index);
                    self.request_collection.save();
                    if index > 0 {
                        self.request_selection_state.select(Some(index - 1));
                    }
                    if self.request_collection.requests.len() == 0 {
                        self.modal = Modal::None;
                    }
                }
            }
            _ => {}
        };
    }

    fn handle_url_input(&mut self, event: KeyEvent) {
        if event.code == KeyCode::Enter {
            self.make_request();
            self.set_view(View::Response);
            return;
        }
        match event.code {
            KeyCode::Right => self.url.handle_command(EditCommand::ForwardCursor),
            KeyCode::Left => self.url.handle_command(EditCommand::BackwardCursor),
            KeyCode::Backspace => self.url.handle_command(EditCommand::BackwardDelete),
            KeyCode::Delete => self.url.handle_command(EditCommand::ForwardDelete),
            KeyCode::Char(c) => self.url.handle_command(EditCommand::InsertCharacter(c)),
            _ => {}
        };
    }

    fn handle_request_body_input(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Right => self.body.handle_command(EditCommand::ForwardCursor),
            KeyCode::Left => self.body.handle_command(EditCommand::BackwardCursor),
            KeyCode::Backspace => self.body.handle_command(EditCommand::BackwardDelete),
            KeyCode::Delete => self.body.handle_command(EditCommand::ForwardDelete),
            KeyCode::Char(c) => self.body.handle_command(EditCommand::InsertCharacter(c)),
            KeyCode::Enter => {
                self.body.handle_command(EditCommand::InsertCharacter('\n'));
            }
            KeyCode::Up => self.body.handle_command(EditCommand::UpCursor),
            KeyCode::Down => self.body.handle_command(EditCommand::DownCursor),
            _ => {}
        };
    }

    fn handle_request_headers_input(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Right => self.headers.handle_command(EditCommand::ForwardCursor),
            KeyCode::Left => self.headers.handle_command(EditCommand::BackwardCursor),
            KeyCode::Backspace => self.headers.handle_command(EditCommand::BackwardDelete),
            KeyCode::Delete => self.headers.handle_command(EditCommand::ForwardDelete),
            KeyCode::Char(c) => self.headers.handle_command(EditCommand::InsertCharacter(c)),
            KeyCode::Enter => {
                self.headers
                    .handle_command(EditCommand::InsertCharacter('\n'));
            }
            KeyCode::Up => self.headers.handle_command(EditCommand::UpCursor),
            KeyCode::Down => self.headers.handle_command(EditCommand::DownCursor),
            _ => {}
        };
    }

    fn reset(&mut self) {
        self.response_paragraph.lock().unwrap().reset();
        self.response_header_paragraph.lock().unwrap().reset();
        *self.response.lock().unwrap() = None;
    }

    pub fn make_request(&mut self) {
        self.reset();
        let sender = self.sender.clone();
        let method = self.method;
        let url = String::from(self.url.as_str());
        let response = self.response.clone();
        let res_paragraph = self.response_paragraph.clone();
        let headers = String::from(self.headers.as_str());
        let body = String::from(self.body.as_str());
        let dirty = self.dirty.clone();
        let response_header_paragraph = self.response_header_paragraph.clone();

        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(10);
            sender
                .send(Request {
                    method,
                    url,
                    headers,
                    resp: tx,
                    body,
                })
                .await
                .unwrap();

            loop {
                let res = rx.recv().await;

                match res {
                    Some(Response::Headers(res)) => {
                        let header_string = jsonxf::pretty_print(format!("{:?}", res).as_str());
                        if let Ok(header_string) = header_string {
                            response_header_paragraph
                                .lock()
                                .unwrap()
                                .set_value(header_string);
                        }
                    }
                    Some(Response::Body(res)) => {
                        let mut response_bytes = response.lock().unwrap();

                        let decoded_string = String::from_utf8_lossy(&res);
                        let pretty_json = jsonxf::pretty_print(decoded_string.to_string().as_str());

                        let final_string = if let Ok(pretty_json) = pretty_json {
                            pretty_json
                        } else {
                            decoded_string.to_string()
                        };

                        *response_bytes = Some(res);
                        res_paragraph.lock().unwrap().set_value(final_string);
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
