use super::util;
use reqwest::Client;

const EVENT_SOURCE: &str = "nws_api";
const DATA_TYPE: &str = "TODO";

#[derive(Serialize)]
pub struct WxEventMessage<'a> {
    pub src: &'a str,
    #[serde(skip)]
    pub key: String,
    pub event_ts: u64,
    pub ingest_ts: u64,
    pub data: String,
    pub data_type: &'a str,
}

impl<'a> WxEventMessage<'a> {
    pub fn new(event_ts: u64) -> WxEventMessage<'a> {
        WxEventMessage {
            src: EVENT_SOURCE,
            key: get_event_key(event_ts),
            event_ts,
            ingest_ts: util::get_system_millis(),
            data: "".to_string(), //serde_json::to_string(data).unwrap(),
            data_type: DATA_TYPE,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Coordinates {
    pub lat: f32,
    pub lon: f32,
}

fn get_event_key(event_ts: u64) -> String {
    format!("nws-{}-{}", event_ts, "TODO")
}

#[derive(Deserialize)]
pub struct ProductsResult {
    #[serde(rename = "@context")]
    pub _context: Context,
    #[serde(rename = "@graph")]
    pub products: Vec<Product>,
}

#[derive(Deserialize)]
pub struct Context {
    #[serde(rename = "@vocab")]
    pub _vocab: String,
}

#[derive(Serialize, Deserialize)]
pub struct Product {
    #[serde(rename = "@id")]
    _id: String,
    pub id: String,
    #[serde(rename = "wmoCollectiveId")]
    wmo_collective_id: String,
    #[serde(rename = "issuingOffice")]
    issuing_office: String,
    #[serde(rename = "issuanceTime")]
    pub issuance_time: String,
    #[serde(rename = "productCode")]
    pub product_code: String,
    #[serde(rename = "productName")]
    product_name: String,
    #[serde(default)]
    #[serde(rename = "productText")]
    pub product_text: Option<String>,
}
