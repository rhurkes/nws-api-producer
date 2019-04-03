use reqwest::Client;
use serde::de::DeserializeOwned;
use std::fs::File;
use std::io::{ErrorKind, Read};
use wx::util::Logger;

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub broker_list: String,
    pub topic_name: String,
    pub consumer_id: String,
    pub api_host: String,
    pub poll_interval_ms: u64,
    pub age_limit_min: u64,
    pub user_agent: String,
}

impl Config {
    pub fn new(filepath: &str) -> Config {
        let mut f = File::open(filepath).expect("config file not found");
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");
        let mut config: Config = toml::from_str(&contents).expect("unable to parse config");

        // let api_url_env = env::var("NWS_API_URL");
        // if api_url_env.is_ok() {
        //     config.api_url = api_url_env.unwrap();
        // }

        // let poll_interval_ms_env = env::var("NWS_POLL_INTERVAL_MS");
        // if poll_interval_ms_env.is_ok() {
        //     config.poll_interval_ms = poll_interval_ms_env.unwrap().parse().unwrap();
        // }

        config
    }
}

pub struct Fetcher<'a> {
    pub client: &'a Client,
    pub config: &'a Config,
    pub logger: &'a Logger,
}

impl<'a> Fetcher<'a> {
    pub fn new(client: &'a Client, config: &'a Config, logger: &'a Logger) -> Fetcher<'a> {
        Fetcher {
            client,
            config,
            logger,
        }
    }

    pub fn fetch<T: DeserializeOwned>(&self, url: &str) -> Result<T, Box<std::error::Error>> {
        let mut response = self
            .client
            .get(url)
            .header(reqwest::header::USER_AGENT, self.config.user_agent.as_str())
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
