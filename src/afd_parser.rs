use super::domain::Product;
use wx::domain::{Event, EventType};
use wx::error::Error;
use wx::util;

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let wfo = product.issuing_office.to_string();
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let title = format!("Area Forecast Discussion ({})", wfo);
    let ext_uri = Some(product._id.to_string());

    let event = Event {
        event_ts,
        event_type: EventType::NwsAfd,
        expires_ts: None,
        ext_uri,
        ingest_ts: 0,
        location: None,
        md: None,
        outlook: None,
        report: None,
        text: None,
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
        let product = get_product_from_file("data/products/afd-mpx");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1523671620000000,"event_type":"NwsAfd","expires_ts":null,"ext_uri":"https://api.weather.gov/products/d0b93b47-1052-4b07-965e-286025226ba8","ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":null,"title":"Area Forecast Discussion (KMPX)","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
