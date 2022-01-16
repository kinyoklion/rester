use crate::{Request, Response};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str;
use std::str::FromStr;
use tokio::sync::mpsc::Receiver;

pub fn web_request_handler(mut receiver: Receiver<Request>) {
    tokio::spawn(async move {
        loop {
            let client = reqwest::Client::new();
            let req = receiver.recv().await;
            // println!("Request {:?}", req);
            match req {
                Some(req) => {
                    info!("Request present");
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
                            match req
                                .resp
                                .send(Response::Headers(res.headers().clone()))
                                .await
                            {
                                Ok(_) => {
                                    let bytes = res.bytes().await;
                                    if let Ok(bytes) = bytes {
                                        if let Err(err) = req.resp.send(Response::Body(bytes)).await
                                        {
                                            error!("Error replying to request {:?}", err);
                                        }
                                    }
                                }
                                Err(err) => {
                                    error!("Error sending request: {:?}", err);
                                    if let Err(err) = req.resp.send(Response::Failure).await {
                                        error!("Error replying to request {:?}", err);
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
                _ => {
                    break;
                }
            };
        }
    });
}
