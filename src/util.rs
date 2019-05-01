use reqwest::Client;
use serde::de::DeserializeOwned;
use std::io::Read;
use wx::error::{Error, WxError};
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

    pub fn fetch<T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        let mut response = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, self.user_agent)
            .send();

        // Do a single retry if the first call fails
        if response.is_err() {
            info!(self.logger, "fetch retry"; "url" => url);
            response = self
                .client
                .get(url)
                .header(reqwest::header::USER_AGENT, self.user_agent)
                .send();
        }

        if response.is_err() {
            let msg = format!("unable to fetch url: {}", url);
            return Err(Error::Wx(<WxError>::new(&msg)));
        }

        let mut response = response.unwrap();
        let status = response.status();

        if status != reqwest::StatusCode::OK {
            let msg = format!(
                "Unexpected status code: {}, url: {}",
                response.status(),
                url
            );
            return Err(Error::Wx(<WxError>::new(&msg)));
        } else {
            debug!(self.logger, "fetch"; "url" => url, "status" => status.to_string());
        }

        let mut body = String::new();
        response.read_to_string(&mut body)?;
        let result: T = serde_json::from_str(&body)?;

        Ok(result)
    }
}
