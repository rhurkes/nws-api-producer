use super::domain::Product;
use super::parser::{cap, get_parse_error, short_time_to_ticks, str_to_latlon, Regexes};
use wx::domain::{Coordinates, Event, EventType, Location, Warning};
use wx::error::Error;
use wx::util;

/**
 * Parses an NWS Flash Flood Warning (FFW).
 */
pub fn parse(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let description = regexes
        .description
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let poly_captures = regexes.poly.captures_iter(&text);
    let valid_range = regexes
        .valid
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let issued_for = regexes
        .issued_for
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let issued_for = cap(issued_for.name("for"))
        .replace("\n", "")
        .replace("  ", " ");
    let issued_for = issued_for.trim().to_string();
    let text = cap(description.name("desc"))
        .to_string()
        .replace("\n", "")
        .replace("  ", " ");

    let mut poly: Vec<Coordinates> = vec![];
    for latlon in poly_captures {
        let splits: Vec<&str> = latlon[0].split(' ').collect();
        poly.push(Coordinates {
            lat: str_to_latlon(splits[0], false),
            lon: str_to_latlon(splits[1], true),
        });
    }

    let wfo = product.issuing_office.to_string();
    let valid_ts = Some(short_time_to_ticks(&valid_range[1])?);
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let expires_ts = Some(short_time_to_ticks(&valid_range[2])?);
    let title = format!("{} issues Flash Flood Warning", wfo); // 31 chars max

    let location = Some(Location {
        wfo: Some(wfo),
        point: None,
        poly: Some(poly),
    });

    let lower_case_text = text.to_lowercase();

    let warning = Some(Warning {
        is_pds: lower_case_text.contains("particularly dangerous situation"),
        was_observed: None,
        is_tor_emergency: None,
        motion_deg: None,
        motion_kt: None,
        source: None,
        issued_for,
        time: "".to_string(),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsFfw,
        expires_ts,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location,
        md: None,
        outlook: None,
        report: None,
        text: Some(text),
        title,
        valid_ts,
        warning,
        watch: None,
    };

    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::super::test_util::get_product_from_file;
    use super::*;

    #[test]
    fn parse_ffw_product_happy_path() {
        let product = get_product_from_file("data/products/ffw");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525225920000000,"event_type":"NwsFfw","expires_ts":1525239900000000,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KGID","point":null,"poly":[{"lat":39.35,"lon":-98.47},{"lat":39.53,"lon":-97.93},{"lat":39.22,"lon":-97.93},{"lat":39.22,"lon":-98.49},{"lat":39.13,"lon":-98.49},{"lat":39.13,"lon":-98.89}]},"md":null,"outlook":null,"report":null,"text":"At 844 PM CDT, Doppler radar indicated thunderstorms producing heavy rain across the warned area. Flash flooding is expected to  begin shortly. Three to five inches of rain have been estimated to  have already fallen for some areas, with potentially another  couple of inches of rain before ending Tuesday night.","title":"KGID issues Flash Flood Warning","valid_ts":1525225920000000,"warning":{"is_pds":false,"is_tor_emergency":null,"was_observed":null,"issued_for":"Mitchell County in north central Kansas... Southeastern Osborne County in north central Kansas...","motion_deg":null,"motion_kt":null,"source":null,"time":""},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
