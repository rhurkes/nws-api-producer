use super::domain::Product;
use super::parser::{cap, get_parse_error, short_time_to_ticks, str_to_latlon, Regexes};
use wx::domain::{Coordinates, Event, EventType, Location, Warning};
use wx::error::Error;
use wx::util;

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
    let title = format!("Tornado Warning ({})", wfo);

    let location = Some(Location {
        wfo: Some(wfo),
        point: Some(Coordinates { lat, lon }),
        poly: Some(poly),
        county: None,
    });

    let lower_case_text = text.to_lowercase();

    let warning = Some(Warning {
        is_pds: lower_case_text.contains("particularly dangerous situation"),
        was_observed: Some(lower_case_text.contains("tornado...observed")),
        is_tor_emergency: Some(lower_case_text.contains("tornado emergency")),
        motion_deg: Some(cap(movement.name("deg")).parse::<u16>()?),
        motion_kt: Some(cap(movement.name("kt")).parse::<u16>()?),
        source: Some(cap(source.name("src")).to_string()),
        issued_for,
        time: cap(movement.name("time")).to_string(),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsTor,
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
    fn parse_tor_product_happy_path() {
        let product = Product{
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "TOR".to_string(),
            product_name: "Tornado Warning".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...RADAR INDICATED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n".to_string(),
        };

        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525222860000000,"event_type":"NwsTor","expires_ts":1525225500000000,"ext_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":{"lat":39.52,"lon":-97.28},"poly":[{"lat":39.77,"lon":-96.97},{"lat":39.5,"lon":-96.8},{"lat":39.39,"lon":-97.37},{"lat":39.59,"lon":-97.37}],"county":null},"md":null,"outlook":null,"report":null,"text":"\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...RADAR INDICATED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n","title":"Tornado Warning (KTOP)","valid_ts":1525222860000000,"warning":{"is_pds":false,"is_tor_emergency":false,"was_observed":false,"issued_for":"Northwestern Riley County in northeastern Kansas, Southern Washington County in north central Kansas, Northern Clay County in north central Kansas","motion_deg":245,"motion_kt":24,"source":"Radar indicated rotation","time":"0100Z"},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_tor_product_non_default_fields() {
        let product = Product{
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "TOR".to_string(),
            product_name: "Tornado Warning".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  THIS IS A TORNADO EMERGENCY FOR CLIFTON. \n\n This is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...OBSERVED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n".to_string(),
        };

        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525222860000000,"event_type":"NwsTor","expires_ts":1525225500000000,"ext_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":{"lat":39.52,"lon":-97.28},"poly":[{"lat":39.77,"lon":-96.97},{"lat":39.5,"lon":-96.8},{"lat":39.39,"lon":-97.37},{"lat":39.59,"lon":-97.37}],"county":null},"md":null,"outlook":null,"report":null,"text":"\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  THIS IS A TORNADO EMERGENCY FOR CLIFTON. \n\n This is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...OBSERVED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n","title":"Tornado Warning (KTOP)","valid_ts":1525222860000000,"warning":{"is_pds":true,"is_tor_emergency":true,"was_observed":true,"issued_for":"Northwestern Riley County in northeastern Kansas, Southern Washington County in north central Kansas, Northern Clay County in north central Kansas","motion_deg":245,"motion_kt":24,"source":"Radar indicated rotation","time":"0100Z"},"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_tor_with_100_lon() {
        let product = get_product_from_file("data/products/tor-normal");
        let regexes = Regexes::new();
        let result = parse(&product, regexes);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_tor_with_long_source() {
        let product = get_product_from_file("data/products/tor-long-source");
        let regexes = Regexes::new();
        let result = parse(&product, regexes);
        assert!(result.is_ok());
    }
}
