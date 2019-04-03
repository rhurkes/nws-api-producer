use super::domain::Product;
use chrono::prelude::*;
use wx::domain::{Coordinates, Event, EventType, HazardType, Location, Report, Units};
use wx::error::{Error, WxError};
use wx::util;

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO finish
    // #[test]
    // fn parse_afd_product() {
    //     let parser = RegexParser::new();
    //     let mut product = Product {
    //         _id: "_id".to_string(),
    //         id: "id".to_string(),
    //         issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
    //         issuing_office: "KTOP".to_string(),
    //         product_code: "AFD".to_string(),
    //         product_name: "Area Forecast Discussion".to_string(),
    //         wmo_collective_id: "WFUS53".to_string(),
    //         product_text: "TODO get some text".to_string(),
    //     };

    //     let result = parser.parse(&mut product).unwrap();
    //     let serialized_result = serde_json::to_string(&result).unwrap();
    //     let expected = "TODO get expected";

    //     assert!(serialized_result == expected);
    // }
}
