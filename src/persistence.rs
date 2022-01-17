use crate::Method;

use serde::{Deserialize, Serialize};

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

impl KeyValuePair {
    pub fn to_string(&self) -> String {
        format!("{:?}:{:?}", self.key, self.value)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub key: String,
    pub method: Method,
    pub url: String,
    // Headers and params are lists for deterministic
    // JSON ordering. Versus an object which an editor may
    // re-order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<KeyValuePair>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Vec<KeyValuePair>>,
}

impl Request {
    pub fn headers_to_string(&self) -> String {
        match &self.headers {
            None => "".to_string(),
            Some(headers) => {
                let strings: Vec<String> = headers.iter().map(|i| i.to_string()).collect();
                strings.join("\r\n")
            }
        }
    }
}

pub struct RequestBuilder {
    pub key: String,
    pub method: Method,
    pub url: Option<String>,
    // Headers and params are lists for deterministic
    // JSON ordering. Versus an object which an editor may
    // re-order.
    pub headers: Option<String>,
    pub params: Option<String>,
}

impl RequestBuilder {
    pub fn new(req_key: &str) -> RequestBuilder {
        RequestBuilder {
            key: req_key.to_string(),
            method: Method::GET,
            url: None,
            headers: None,
            params: None,
        }
    }

    pub fn headers(&mut self, headers: &str) -> &Self {
        self.headers = Some(headers.to_string());
        self
    }

    pub fn url(&mut self, url: &str) -> &Self {
        self.url = Some(url.to_string());
        self
    }

    pub fn method(&mut self, method: Method) -> &Self {
        self.method = method;
        self
    }

    pub fn build(self) -> Request {
        let headers = match self.headers {
            None => None,
            Some(header_string) => {
                let headers_strings: Vec<&str> = header_string.split("\r\n").collect();
                let mut parsed_headers: Vec<KeyValuePair> = Vec::new();

                for entry in headers_strings {
                    if let Some((key, value)) = entry.split_once(":") {
                        parsed_headers.push(KeyValuePair {
                            key: key.trim().to_string(),
                            value: value.trim().to_string(),
                        })
                    }
                }
                if parsed_headers.is_empty() {
                    None
                } else {
                    Some(parsed_headers)
                }
            }
        };

        Request {
            key: self.key,
            method: Method::GET,
            url: self.url.expect("Must set URL."),
            headers,
            params: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestCollection {
    pub requests: Vec<Request>,
}

impl RequestCollection {
    pub fn new() -> Self {
        RequestCollection {
            requests: Vec::new(),
        }
    }

    pub fn add_request(&mut self, request: Request) {
        let req_key = request.key.as_str();
        match self.requests.iter().position(|item| item.key == req_key) {
            None => {
                self.requests.push(request);
            }
            Some(index) => {
                let _ = std::mem::replace(&mut self.requests[index], request);
            }
        };
    }

    pub fn save(&self) {
        let serialized = serde_json::to_string_pretty(&self.requests);
        info!("Serialized: {:?}", serialized);
        let file = File::create("requests.json");
        if let Ok(mut file) = file {
            if let Err(err) = file.write_all(serialized.unwrap().as_bytes()) {
                error!("Error writing file {:?}", err);
            }
        }
    }

    pub fn load() -> Self {
        if Path::new("requests.json").exists() {
            match File::open("requests.json") {
                Ok(file) => {
                    let reader = BufReader::new(file);

                    // Read the JSON contents of the file as an instance of `User`.
                    match serde_json::from_reader(reader) {
                        Ok(collection) => {
                            return Self {
                                requests: collection,
                            };
                        }
                        _ => {}
                    }
                }
                Err(_) => {}
            }
        }
        Self::new()
    }
}
