use super::domain::Product;
use chrono::prelude::*;
use super::parser::{cap, get_parse_error, short_time_to_ticks, str_to_latlon};
use wx::domain::{Coordinates, Event, EventType, HazardType, Location, Report, Units, Warning};
use wx::error::{Error, WxError};
use wx::util;

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let movement = self
        .movement_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let description = self
        .description_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let poly = self
        .polygon_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let source = self
        .source_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let lat = str_to_latlon(cap(movement.name("lat")), false);
    let lon = str_to_latlon(cap(movement.name("lon")), true);
    let poly = cap(poly.name("poly"));
    let valid_range = self
        .valid_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let issued_for = self
        .issued_for_regex
        .captures(&text)
        .ok_or(get_parse_error(&text))?;
    let issued_for = cap(issued_for.name("for")).replace("\n", "");
    let issued_for = issued_for.trim().to_string();
    let summary = cap(description.name("desc")).to_string();

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
    let title = format!("{} issues tornado warning", wfo); // 31 chars max

    let location = Some(Location {
        wfo: Some(wfo),
        point: Some(Coordinates { lat, lon }),
        poly: Some(poly),
    });

    let warning = Some(Warning {
        is_pds: text.contains("particularly dangerous situation"),
        was_observed: text.contains("tornado...observed"),
        is_tor_emergency: text.contains("tornado emergency"),
        motion_deg: cap(movement.name("deg")).parse::<u16>()?,
        motion_kt: cap(movement.name("kt")).parse::<u16>()?,
        source: cap(source.name("src")).to_string(),
        issued_for,
        time: cap(movement.name("time")).to_string(),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsTor,
        expires_ts,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location,
        md: None,
        outlook: None,
        report: None,
        summary,
        text: None,
        title,
        valid_ts,
        warning,
        watch: None,
    };

    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tor_product() {
        let parser = RegexParser::new();
        let mut product = Product{
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "TOR".to_string(),
            product_name: "Tornado Warning".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  This is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...RADAR INDICATED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n".to_string(),
        };

        let result = parser.parse(&mut product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = "{\"event_ts\":1525222860000,\"event_type\":\"NwsTor\",\"expires_ts\":1525225500000,\"fetch_status\":null,\"image_uri\":null,\"ingest_ts\":0,\"location\":{\"wfo\":\"KTOP\",\"point\":{\"lat\":39.52,\"lon\":-97.28},\"poly\":[{\"lat\":39.77,\"lon\":-96.97},{\"lat\":39.5,\"lon\":-96.8},{\"lat\":39.39,\"lon\":-97.37},{\"lat\":39.59,\"lon\":-97.37}]},\"md\":null,\"outlook\":null,\"report\":null,\"summary\":\"At 800 PM CDT, a large and extremely dangerous tornado was located\\n  2 miles south of Clifton, moving northeast at 25 mph.\",\"text\":null,\"title\":\"KTOP issues tornado warning\",\"valid_ts\":1525222860000,\"warning\":{\"is_pds\":false,\"is_tor_emergency\":false,\"was_observed\":false,\"issued_for\":\"Northwestern Riley County in northeastern Kansas...  Southern Washington County in north central Kansas...  Northern Clay County in north central Kansas...\",\"motion_deg\":245,\"motion_kt\":24,\"source\":\"Radar indicated rotation\",\"time\":\"0100Z\"},\"watch\":null}";
        assert!(serialized_result == expected);
    }
}
