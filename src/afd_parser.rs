use super::domain::Product;
use wx::domain::{Event, EventType};
use wx::error::Error;
use wx::util;

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let wfo = product.issuing_office.to_string();
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let title = format!("{} issues Area Forecast Discussion", wfo); // 31 chars max
    let text = &product.product_text;

    let event = Event {
        event_ts,
        event_type: EventType::NwsAfd,
        expires_ts: None,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location: None,
        md: None,
        outlook: None,
        report: None,
        text: Some(text.to_string()),
        title,
        valid_ts: None,
        warning: None,
        watch: None,
    };

    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_afd_product() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "AFD".to_string(),
            product_name: "Area Forecast Discussion".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "some text".to_string(),
        };

        let result = parse(&mut product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525222860000000,"event_type":"NwsAfd","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"some text","title":"KTOP issues Area Forecast Discussion","valid_ts":null,"warning":null,"watch":null}"#;
        assert!(serialized_result == expected);
    }
}
