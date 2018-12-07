use chrono::prelude::*;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{BaseProducer, BaseRecord};
use reqwest::Client;
use serde::de::DeserializeOwned;
use slog::Drain;
use std::fs::File;
use std::io::{ErrorKind, Read};
use std::ops::Deref;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub struct Logger {
    pub instance: slog::Logger,
}

impl Logger {
    pub fn new() -> Logger {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();

        Logger {
            instance: slog::Logger::root(drain, o!()),
        }
    }
}

impl Deref for Logger {
    type Target = slog::Logger;
    fn deref(&self) -> &Self::Target {
        &self.instance
    }
}

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

pub struct KafkaProducer<'a> {
    pub logger: &'a Logger,
    pub producer: BaseProducer,
    pub topic_name: &'a str,
}

impl<'a> KafkaProducer<'a> {
    pub fn new(logger: &'a Logger, config: &'a Config) -> KafkaProducer<'a> {
        let producer: BaseProducer = ClientConfig::new()
            .set("group.id", &config.consumer_id)
            .set("bootstrap.servers", &config.broker_list)
            .create()
            .expect("producer creation error");

        KafkaProducer {
            logger,
            producer,
            topic_name: &config.topic_name,
        }
    }

    pub fn write_to_topic(&self, message: &str, key: &str) {
        self.producer
            .send(BaseRecord::to(&self.topic_name).payload(message).key(key))
            .unwrap();

        if self.producer.poll(Duration::from_millis(5000)) == 0 {
            warn!(self.logger, "kafka write not acked within threshold"; "topic" => self.topic_name, "msg" => message );
        }
    }
}

pub fn get_system_millis() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
    since_the_epoch.as_secs() * 1000 + u64::from(since_the_epoch.subsec_nanos()) / 1_000_000
}

pub fn ts_to_ticks(input: &str) -> Result<u64, Box<std::error::Error>> {
    Ok(Utc
        .datetime_from_str(input, "%Y-%m-%dT%H:%M:%S+00:00")?
        .timestamp() as u64
        * 1000)
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
        let mut response = self.client
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_to_ticks_should_return_ticks() {
        let ts = "2018-11-25T22:46:00+00:00";
        let result = ts_to_ticks(&ts);
        assert!(result.unwrap() == 1543185960000);
    }
}
