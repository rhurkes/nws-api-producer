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
    let poly_captures = regexes.poly.captures_iter(&text);
    let source = regexes
        .source
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let lat = str_to_latlon(cap(movement.name("lat")), false);
    let lon = str_to_latlon(cap(movement.name("lon")), true);
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
    for latlon in poly_captures.take(4) {
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
    let title = format!("Severe Thunderstorm Warning ({})", wfo); // 31 chars max

    let location = Some(Location {
        wfo: Some(wfo),
        point: Some(Coordinates { lat, lon }),
        poly: Some(poly),
        county: None,
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
        ext_uri: None,
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
    fn parse_svr_product_happy_path() {
        let product = get_product_from_file("data/products/svr");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1523658960000000,"event_type":"NwsSvr","expires_ts":1523661300000000,"ext_uri":null,"ingest_ts":0,"location":{"wfo":"KDMX","point":{"lat":41.98,"lon":-94.62},"poly":[{"lat":42.21,"lon":-94.75},{"lat":42.21,"lon":-94.34},{"lat":41.91,"lon":-94.52},{"lat":41.91,"lon":-94.75}],"county":null},"md":null,"outlook":null,"report":null,"text":"\n601 \nWUUS53 KDMX 132236\nSVRDMX\nIAC027-073-132315-\n/O.NEW.KDMX.SV.W.0002.180413T2236Z-180413T2315Z/\n\nBULLETIN - IMMEDIATE BROADCAST REQUESTED\nSevere Thunderstorm Warning\nNational Weather Service Des Moines IA\n536 PM CDT FRI APR 13 2018\n\nThe National Weather Service in Des Moines  has issued a\n\n* Severe Thunderstorm Warning for...\n  Western Greene County in west central Iowa...\n  Eastern Carroll County in west central Iowa...\n\n* Until 615 PM CDT.\n\n* At 536 PM CDT, a severe thunderstorm was located 7 miles southeast\n  of Glidden, or 12 miles west of Jefferson, moving northeast at 30\n  mph.\n\n  HAZARD...60 mph wind gusts and quarter size hail. \n\n  SOURCE...Radar indicated. \n\n  IMPACT...Hail damage to vehicles is expected. Expect wind damage \n           to roofs, siding, and trees. \n\n* Locations impacted include...\n  Glidden, Scranton, Churdan, Lanesboro, Ralston and Hobbs County\n  Park.\n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nFor your protection move to an interior room on the lowest floor of a\nbuilding.\n\nTorrential rainfall is occurring with this storm, and may lead to\nflash flooding. Do not drive your vehicle through flooded roadways.\n\n&&\n\nLAT...LON 4221 9475 4221 9434 4191 9452 4191 9475\nTIME...MOT...LOC 2236Z 206DEG 24KT 4198 9462 \n\nHAIL...1.00IN\nWIND...60MPH\n \n$$\n\nMF\n\n","title":"Severe Thunderstorm Warning (KDMX)","valid_ts":1523658960000000,"warning":{"is_pds":false,"is_tor_emergency":null,"was_observed":null,"issued_for":"Western Greene County in west central Iowa, Eastern Carroll County in west central Iowa","motion_deg":206,"motion_kt":24,"source":"Radar indicated","time":"2236Z"},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
