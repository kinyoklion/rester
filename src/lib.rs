use bytes::Bytes;
use reqwest::header::HeaderMap;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use strum_macros::IntoStaticStr;
use tokio::sync::mpsc;
#[macro_use]
extern crate log;

pub mod app;
pub mod default_key_binds;
pub mod key_bind;
pub mod layout;
pub mod paragraph_with_state;
pub mod persistence;
pub mod ui;
pub mod web_request_handler;

pub type Responder<T> = mpsc::Sender<T>;

#[derive(Copy, Clone, PartialEq, IntoStaticStr, Debug, Serialize, Deserialize)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Debug)]
pub enum Response {
    Status(StatusCode),
    Headers(HeaderMap),
    Body(Bytes),
    Failure,
}

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: String,
    pub body: String,
    pub resp: Responder<Response>,
}

#[derive(Debug)]
pub enum WebRequest {
    Request(Request),
    Cancel,
}

#[derive(Debug)]
pub enum ScrollDirection {
    Up,
    Down,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Operation {
    GotoUrl,
    GotoRequestBody,
    GotoRequestHeaders,
    GotoResponseBody,
    GotoResponseHeaders,
    NextMethod,
    LoadRequest,
    SaveRequest,
    SaveResponse,
    GotoRequestView,
    GotoResponseView,
    SendRequest,
    Quit,
}
