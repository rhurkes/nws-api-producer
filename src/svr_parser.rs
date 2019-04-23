use super::domain::Product;
use super::parser::{cap, get_parse_error, short_time_to_ticks, str_to_latlon, Regexes};
use wx::domain::{Coordinates, Event, EventType, Location, Warning};
use wx::error::Error;
use wx::util;

/**
 * Parses an NWS Severe Thunderstorm Warning (SVR).
 */
pub fn parse(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let movement = regexes
        .movement
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let description = regexes
        .description
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let poly = regexes
        .four_node_poly
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let source = regexes
        .source
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let lat = str_to_latlon(cap(movement.name("lat")), false);
    let lon = str_to_latlon(cap(movement.name("lon")), true);
    let poly = cap(poly.name("poly"));
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

    let poly = vec![
        Coordinates {
            lat: str_to_latlon(&poly[0..4], false),
            lon: str_to_latlon(&poly[5..9], true),
        },
        Coordinates {
            lat: str_to_latlon(&poly[10..14], false),
            lon: str_to_latlon(&poly[15..19], true),
        },
        Coordinates {
            lat: str_to_latlon(&poly[20..24], false),
            lon: str_to_latlon(&poly[25..29], true),
        },
        Coordinates {
            lat: str_to_latlon(&poly[30..34], false),
            lon: str_to_latlon(&poly[35..39], true),
        },
    ];

    let wfo = product.issuing_office.to_string();
    let valid_ts = Some(short_time_to_ticks(&valid_range[1])?);
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let expires_ts = Some(short_time_to_ticks(&valid_range[2])?);
    let title = format!("{} issues Sev Tstm Warning", wfo); // 31 chars max

    let location = Some(Location {
        wfo: Some(wfo),
        point: Some(Coordinates { lat, lon }),
        poly: Some(poly),
    });

    let lower_case_text = text.to_lowercase();

    let warning = Some(Warning {
        is_pds: lower_case_text.contains("particularly dangerous situation"),
        was_observed: None,
        is_tor_emergency: None,
        motion_deg: Some(cap(movement.name("deg")).parse::<u16>()?),
        motion_kt: Some(cap(movement.name("kt")).parse::<u16>()?),
        source: Some(cap(source.name("src")).to_string()),
        issued_for,
        time: cap(movement.name("time")).to_string(),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsSvr,
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
    fn parse_svr_product_happy_path() {
        let product = get_product_from_file("data/products/svr");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1523658960000000,"event_type":"NwsSvr","expires_ts":1523661300000000,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KDMX","point":{"lat":41.98,"lon":-94.62},"poly":[{"lat":42.21,"lon":-94.75},{"lat":42.21,"lon":-94.34},{"lat":41.91,"lon":-94.52},{"lat":41.91,"lon":-94.75}]},"md":null,"outlook":null,"report":null,"text":"At 536 PM CDT, a severe thunderstorm was located 7 miles southeast of Glidden, or 12 miles west of Jefferson, moving northeast at 30 mph.","title":"KDMX issues Sev Tstm Warning","valid_ts":1523658960000000,"warning":{"is_pds":false,"is_tor_emergency":null,"was_observed":null,"issued_for":"Western Greene County in west central Iowa... Eastern Carroll County in west central Iowa...","motion_deg":206,"motion_kt":24,"source":"Radar indicated","time":"2236Z"},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
