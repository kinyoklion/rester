use crate::paragraph_with_state::ParagraphWithState;
use crate::persistence::RequestCollection;
use std::fs::File;
use std::io::Write;

use crate::{default_key_binds, Method, Operation, Request, Response};
use bytes::Bytes;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::key_bind::KeyBind;
use crate::ui::text_area::{EditCommand, EditState};
use reqwest::header::HeaderValue;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
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
    pub key_binds: Vec<KeyBind>,
    pub status: Arc<AtomicU16>,
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
            key_binds: default_key_binds::default_key_binds(),
            status: Arc::new(AtomicU16::new(0)),
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

    fn handle_operation(&mut self, operation: Operation) -> bool {
        match operation {
            Operation::GotoUrl => {
                self.mode = Mode::Url;
            }
            Operation::GotoRequestBody => {
                self.set_view(View::Request);
                self.mode = Mode::RequestBody;
            }
            Operation::GotoRequestHeaders => {
                self.set_view(View::Request);
                self.mode = Mode::RequestHeaders;
            }
            Operation::GotoResponseBody => {
                self.set_view(View::Response);
                self.mode = Mode::ResponseBody;
            }
            Operation::GotoResponseHeaders => {
                self.set_view(View::Response);
                self.mode = Mode::ResponseHeaders
            }
            Operation::NextMethod => {
                self.next_method();
            }
            Operation::LoadRequest => {
                if self.modal == Modal::None {
                    self.modal = Modal::Requests;
                    self.request_selection_state.select(Some(0));
                }
            }
            Operation::SaveRequest => {
                if self.modal == Modal::None {
                    self.modal = Modal::Save;
                }
            }
            Operation::SaveResponse => {
                let resp = self.response_paragraph.lock();
                let para = &*resp.unwrap();

                let url = self.url.as_str().to_string();
                let url = url.replace("://", "_");
                let url = url.replace("/", "_");
                let url = url.replace(":", "_");
                let mut filename = sanitize_filename::sanitize(url);
                filename.push_str(".txt");

                let file = File::create(filename);
                if let Ok(mut file) = file {
                    if let Err(err) = file.write_all(para.as_str().as_bytes()) {
                        error!("Error writing file {:?}", err);
                    }
                }
            }
            Operation::GotoRequestView => {
                self.set_view(View::Request);
            }
            Operation::GotoResponseView => {
                self.set_view(View::Response);
            }
            Operation::SendRequest => {
                self.make_request();
                self.set_view(View::Response);
            }
            Operation::Quit => {
                return true;
            }
        };
        false
    }

    pub fn handle_input(&mut self, key: KeyEvent) -> bool {
        info!("Handling {:?}", key);
        let key_bind = self
            .key_binds
            .iter()
            .find(|key_bind| key_bind.key == key.code && key.modifiers == key_bind.modifiers);

        if let Some(key_bind) = key_bind {
            let operation = key_bind.operation.clone();
            return self.handle_operation(operation);
        }

        if key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
        {
            return false;
        }
        match key.code {
            KeyCode::Esc => {
                return if self.modal == Modal::None {
                    false
                } else {
                    self.modal = Modal::None;
                    false
                }
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
        builder.body(self.body.as_str());
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
                if let Some(body) = &request.body {
                    self.body.set_value(body.clone());
                }

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
        let app_status = self.status.clone();

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

            let mut content_type = "text/plain".to_string();

            loop {
                let res = rx.recv().await;

                match res {
                    Some(Response::Status(status)) => {
                        app_status.store(status.as_u16(), Ordering::SeqCst);
                    }
                    Some(Response::Headers(res)) => {
                        let header_string = jsonxf::pretty_print(format!("{:?}", res).as_str());
                        content_type = res
                            .get("content-type")
                            .unwrap_or(&HeaderValue::from_str(content_type.as_str()).unwrap())
                            .to_str()
                            .unwrap_or("text/plain")
                            .to_string();
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
                        info!("Decoded {:}", decoded_string);
                        let final_string = if content_type.contains("json") {
                            info!("IS JSON");
                            if let Ok(pretty_json) = pretty_json {
                                pretty_json
                            } else {
                                decoded_string.to_string()
                            }
                        } else {
                            decoded_string.to_string()
                        };
                        // let final_string = if let Ok(pretty_json) = pretty_json {
                        //     pretty_json
                        // } else {
                        //     decoded_string.to_string()
                        // };
                        // let final_string = decoded_string.to_string();

                        *response_bytes = Some(res);
                        res_paragraph.lock().unwrap().append_value(final_string);
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
