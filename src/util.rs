use reqwest::Client;
use serde::de::DeserializeOwned;
use std::io::{ErrorKind, Read};
use wx::util::Logger;

pub struct Fetcher<'a> {
    pub client: &'a Client,
    pub logger: &'a Logger,
    pub user_agent: &'a str,
}

impl<'a> Fetcher<'a> {
    pub fn new(client: &'a Client, logger: &'a Logger, user_agent: &'a str) -> Fetcher<'a> {
        Fetcher {
            client,
            logger,
            user_agent,
        }
    }

    pub fn fetch<T: DeserializeOwned>(&self, url: &str) -> Result<T, Box<std::error::Error>> {
        let mut response = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, self.user_agent)
            .send()?;

        let status = response.status();
        debug!(self.logger, "fetch_body"; "url" => url, "status" => status.to_string());

        if status != reqwest::StatusCode::OK {
            let msg = format!("Unexpected status code: {}", response.status());
            return Err(Box::new(std::io::Error::new(ErrorKind::Other, msg)));
        }

        let mut body = String::new();
        response.read_to_string(&mut body)?;
        let result: T = serde_json::from_str(&body)?;

        Ok(result)
    }
}
