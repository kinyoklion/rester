use crate::WebRequest::{Cancel, Request};
use crate::{Method, Response, WebRequest};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str;
use std::str::FromStr;
use tokio::select;
use tokio::sync::mpsc::Receiver;

pub fn web_request_handler(mut receiver: Receiver<WebRequest>) {
    tokio::spawn(async move {
        loop {
            let client = reqwest::Client::new();
            let req = receiver.recv().await;
            match req {
                Some(Request(req)) => {
                    info!("Request present");
                    let mut header_map = HeaderMap::new();
                    let headers: Vec<&str> = req.headers.split("\n").collect();

                    for entry in headers {
                        if let Some((key, value)) = entry.split_once(":") {
                            if let Ok(value) = HeaderValue::from_str(value.trim()) {
                                if let Ok(key) = HeaderName::from_str(key.trim()) {
                                    header_map.append(key, value);
                                }
                            }
                        }
                    }

                    let mut req_builder = match req.method {
                        Method::GET => client.get(req.url).headers(header_map),
                        Method::POST => client.post(req.url).headers(header_map),
                        Method::PUT => client.put(req.url).headers(header_map),
                        Method::DELETE => client.delete(req.url).headers(header_map),
                        Method::PATCH => client.patch(req.url).headers(header_map),
                    };

                    if !req.body.is_empty() {
                        req_builder = req_builder.body(req.body)
                    }
                    let res = req_builder.send().await;
                    match res {
                        Ok(mut res) => {
                            let _ = req.resp.send(Response::Status(res.status())).await;
                            let _ = req
                                .resp
                                .send(Response::Headers(res.headers().clone()))
                                .await;

                            loop {
                                let bytes_future = res.chunk();
                                let request_op = receiver.recv();

                                select! {
                                    in_bytes = bytes_future => {
                                        if let Ok(Some(bytes)) = in_bytes {
                                            if let Err(err) = req.resp.send(Response::Body(bytes)).await {
                                                error!("Error replying to request {:?}", err);
                                                break;
                                            }
                                        } else {
                                            break;
                                        }
                                    },
                                    _request = request_op => {
                                        // This will likely be a cancel request, but we don't care
                                        // about the content. The signal is enough to know we need
                                        // to move on.
                                        break;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            if let Err(err) = req.resp.send(Response::Failure).await {
                                error!("Error replying to request {:?}", err);
                            }
                        }
                    };
                }
                Some(Cancel) => {
                    continue;
                }
                _ => {
                    break;
                }
            };
        }
    });
}
