use super::domain::Product;
use super::parser::{get_parse_error, short_time_to_ticks, str_to_latlon, Regexes};
use wx::domain::{Coordinates, Event, EventType, Location, Warning};
use wx::error::Error;
use wx::util;

/**
 * Parses an NWS Flash Flood Warning (FFW).
 */
pub fn parse(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let poly_captures = regexes.poly.captures_iter(&text);
    let valid_range = regexes
        .valid
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let issued_for = regexes
        .warning_for
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let issued_for = issued_for[1]
        .replace("\n", "")
        .replace("...", ",")
        .replace("  ", " ");
    let issued_for = issued_for.trim();
    let if_len = issued_for.len() - 1;
    let issued_for = issued_for[..if_len].to_string();

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
    let title = format!("Flash Flood Warning ({})", wfo); // 31 chars max

    let location = Some(Location {
        wfo: Some(wfo),
        point: None,
        poly: Some(poly),
        county: None,
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
        time: "N/A".to_string(),
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
        text: Some(text.to_string()),
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
        let expected = r#"{"event_ts":1525225920000000,"event_type":"NwsFfw","expires_ts":1525239900000000,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KGID","point":null,"poly":[{"lat":39.35,"lon":-98.47},{"lat":39.53,"lon":-97.93},{"lat":39.22,"lon":-97.93},{"lat":39.22,"lon":-98.49},{"lat":39.13,"lon":-98.49},{"lat":39.13,"lon":-98.89}],"county":null},"md":null,"outlook":null,"report":null,"text":"\n500 \nWGUS53 KGID 020152\nFFWGID\nKSC123-141-020545-\n/O.NEW.KGID.FF.W.0001.180502T0152Z-180502T0545Z/\n/00000.0.ER.000000T0000Z.000000T0000Z.000000T0000Z.OO/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nFlash Flood Warning\nNational Weather Service Hastings NE\n852 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Hastings has issued a\n\n* Flash Flood Warning for...\n  Mitchell County in north central Kansas...\n  Southeastern Osborne County in north central Kansas...\n\n* Until 1245 AM CDT\n\n* At 844 PM CDT, Doppler radar indicated thunderstorms producing\n  heavy rain across the warned area. Flash flooding is expected to \n  begin shortly. Three to five inches of rain have been estimated to \n  have already fallen for some areas, with potentially another \n  couple of inches of rain before ending Tuesday night.\n\n* Some locations that will experience flooding include...\n  Beloit, Tipton, Asherville, Simpson, Hunter and Victor and along \n  the Solomon River. \n\nLAT...LON 3935 9847 3953 9793 3922 9793 3922 9849\n      3913 9849 3913 9889\n\n$$\n\nHeinlein\n\n","title":"Flash Flood Warning (KGID)","valid_ts":1525225920000000,"warning":{"is_pds":false,"is_tor_emergency":null,"was_observed":null,"issued_for":"Mitchell County in north central Kansas, Southeastern Osborne County in north central Kansas","motion_deg":null,"motion_kt":null,"source":null,"time":"N/A"},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
