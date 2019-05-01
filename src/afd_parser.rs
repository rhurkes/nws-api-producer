use super::domain::Product;
use wx::domain::{Event, EventType};
use wx::error::Error;
use wx::util;

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let wfo = product.issuing_office.to_string();
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let title = format!("Area Forecast Discussion ({})", wfo); // 31 chars max
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
    use crate::test_util::get_product_from_file;

    #[test]
    fn parse_afd_product() {
        let mut product = get_product_from_file("data/products/afd-mpx");
        product.product_text = "test data".to_string();
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1523671620000000,"event_type":"NwsAfd","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"test data","title":"Area Forecast Discussion (KMPX)","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
