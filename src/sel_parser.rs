use super::domain::Product;
use super::parser::{get_parse_error, Regexes};
use wx::domain::{Event, EventType, Watch, WatchStatus, WatchType};
use wx::error::Error;
use wx::util;

pub fn parse(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let lower_case_text = text.to_lowercase();
    let is_pds = lower_case_text.contains("particularly dangerous situation");
    let id = regexes
        .watch_id
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let id = id[1].parse::<u16>()?;
    let mut issued_for = None;
    let raw_issued_for = regexes.watch_for.captures(&text);

    if raw_issued_for.is_some() {
        let raw_issued_for = raw_issued_for.unwrap();
        let raw_issued_for = raw_issued_for[1].trim();
        let raw_issued_for = raw_issued_for.replace("\n  ", ", ");
        issued_for = Some(raw_issued_for);
    }

    let status = if lower_case_text.contains("storm prediction center has issued") {
        WatchStatus::Issued
    } else if lower_case_text.contains("storm prediction center has cancelled") {
        WatchStatus::Cancelled
    } else {
        WatchStatus::Unknown
    };

    let watch_type = if lower_case_text.contains("tornado watch number") {
        WatchType::Tornado
    } else if lower_case_text.contains("severe thunderstorm watch number") {
        WatchType::SevereThunderstorm
    } else {
        WatchType::Other
    };

    let verb = &match status {
        WatchStatus::Issued => "issues ",
        WatchStatus::Cancelled => "cancels ",
        _ => "",
    };
    let pds_text = if is_pds { "PDS " } else { "" };
    let watch_type_text = &match watch_type {
        WatchType::Tornado => "Tor ",
        WatchType::SevereThunderstorm => "Tstm ",
        _ => "",
    };
    let title = format!("SPC {}{}{}Watch {}", verb, pds_text, watch_type_text, id);

    let watch = Some(Watch {
        is_pds,
        id,
        issued_for,
        watch_type,
        status,
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsSel,
        expires_ts: None,
        ext_uri: None,
        ingest_ts: 0,
        location: None,
        md: None,
        outlook: None,
        report: None,
        text: Some(text.to_string()),
        title,
        valid_ts: None,
        warning: None,
        watch,
    };

    Ok(Some(event))
}

#[cfg(test)]
mod tests {
    use super::super::test_util::get_product_from_file;
    use super::*;

    #[test]
    fn parse_tornado_watch_issued() {
        let product = get_product_from_file("data/products/sel-tor-watch-issued");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522775580000000,"event_type":"NwsSel","expires_ts":null,"ext_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"\n281 \nWWUS20 KWNS 031713\nSEL6  \nSPC WW 031713\nINZ000-KYZ000-OHZ000-040000-\n\nURGENT - IMMEDIATE BROADCAST REQUESTED\nTornado Watch Number 26\nNWS Storm Prediction Center Norman OK\n115 PM EDT Tue Apr 3 2018\n\nThe NWS Storm Prediction Center has issued a\n\n* Tornado Watch for portions of \n  Southern and Central Indiana\n  Northern Kentucky\n  Western and Central Ohio\n\n* Effective this Tuesday afternoon and evening from 115 PM until\n  800 PM EDT.\n\n* Primary threats include...\n  A few tornadoes likely with a couple intense tornadoes possible\n  Scattered damaging wind gusts to 70 mph likely\n  Scattered large hail and isolated very large hail events to 2\n    inches in diameter possible\n\nSUMMARY...Thunderstorms are intensifying along the IL/IN border, and\nwill track eastward across the watch area through the afternoon. \nConditions appear favorable for supercell storms capable of large\nhail, damaging winds, and perhaps a strong tornado or two.\n\nThe tornado watch area is approximately along and 70 statute miles\nnorth and south of a line from 40 miles south southwest of Terre\nHaute IN to 20 miles south southeast of Columbus OH. For a complete\ndepiction of the watch see the associated watch outline update\n(WOUS64 KWNS WOU6).\n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nREMEMBER...A Tornado Watch means conditions are favorable for\ntornadoes and severe thunderstorms in and close to the watch\narea. Persons in these areas should be on the lookout for\nthreatening weather conditions and listen for later statements\nand possible warnings.\n\n&&\n\nOTHER WATCH INFORMATION...CONTINUE...WW 25...\n\nAVIATION...Tornadoes and a few severe thunderstorms with hail\nsurface and aloft to 2 inches. Extreme turbulence and surface wind\ngusts to 60 knots. A few cumulonimbi with maximum tops to 450. Mean\nstorm motion vector 24035.\n\n...Hart\n\n","title":"SPC issues Tor Watch 26","valid_ts":null,"warning":null,"watch":{"is_pds":false,"id":26,"watch_type":"Tornado","status":"Issued","issued_for":"Southern and Central Indiana, Northern Kentucky, Western and Central Ohio"}}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_pds_tornado_watch_issued() {
        let product = get_product_from_file("data/products/sel-tor-pds-watch");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1523645220000000,"event_type":"NwsSel","expires_ts":null,"ext_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"\n136 \nWWUS20 KWNS 131847\nSEL0  \nSPC WW 131847\nARZ000-LAZ000-OKZ000-TXZ000-140300-\n\nURGENT - IMMEDIATE BROADCAST REQUESTED\nTornado Watch Number 40\nNWS Storm Prediction Center Norman OK\n150 PM CDT Fri Apr 13 2018\n\nThe NWS Storm Prediction Center has issued a\n\n* Tornado Watch for portions of \n  Much of Arkansas\n  Northwest Louisiana\n  Southeast Oklahoma\n  Northeast Texas\n\n* Effective this Friday afternoon and evening from 150 PM until\n  1000 PM CDT.\n\n...THIS IS A PARTICULARLY DANGEROUS SITUATION...\n\n* Primary threats include...\n  Numerous tornadoes expected with a few intense tornadoes likely\n  Widespread large hail and isolated very large hail events to 2.5\n    inches in diameter likely\n  Widespread damaging wind gusts to 70 mph likely\n\nSUMMARY...Intense thunderstorms are expected to track across the\nwatch area this afternoon and early evening, posing a risk of\ntornadoes, large hail and damaging winds.  Strong tornadoes are\npossible.  Multiple rounds of severe storms are expected across this\nregion.\n\nThe tornado watch area is approximately along and 70 statute miles\neast and west of a line from 70 miles south of Longview TX to 20\nmiles northeast of Flippin AR. For a complete depiction of the watch\nsee the associated watch outline update (WOUS64 KWNS WOU0).\n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nREMEMBER...A Tornado Watch means conditions are favorable for\ntornadoes and severe thunderstorms in and close to the watch\narea. Persons in these areas should be on the lookout for\nthreatening weather conditions and listen for later statements\nand possible warnings.\n\n&&\n\nOTHER WATCH INFORMATION...CONTINUE...WW 39...\n\nAVIATION...Tornadoes and a few severe thunderstorms with hail\nsurface and aloft to 2.5 inches. Extreme turbulence and surface wind\ngusts to 60 knots. A few cumulonimbi with maximum tops to 500. Mean\nstorm motion vector 24035.\n\n...Hart\n\n","title":"SPC issues PDS Tor Watch 40","valid_ts":null,"warning":null,"watch":{"is_pds":true,"id":40,"watch_type":"Tornado","status":"Issued","issued_for":"Much of Arkansas, Northwest Louisiana, Southeast Oklahoma, Northeast Texas"}}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_severe_thunderstorm_watch_issued() {
        let product = get_product_from_file("data/products/sel-svr-watch");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522768980000000,"event_type":"NwsSel","expires_ts":null,"ext_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"\n727 \nWWUS20 KWNS 031523\nSEL5  \nSPC WW 031523\nARZ000-LAZ000-OKZ000-TXZ000-032300-\n\nURGENT - IMMEDIATE BROADCAST REQUESTED\nSevere Thunderstorm Watch Number 25\nNWS Storm Prediction Center Norman OK\n1025 AM CDT Tue Apr 3 2018\n\nThe NWS Storm Prediction Center has issued a\n\n* Severe Thunderstorm Watch for portions of \n  Southwest Arkansas\n  Northwest Louisiana\n  Southeast Oklahoma\n  Central and Northeast Texas\n\n* Effective this Tuesday morning and evening from 1025 AM until\n  600 PM CDT.\n\n* Primary threats include...\n  Scattered large hail likely with isolated very large hail events\n    to 2.5 inches in diameter possible\n  Scattered damaging wind gusts to 70 mph possible\n\nSUMMARY...Thunderstorms are intensifying over central Texas, and\nwill spread northeastward across the watch area through the\nafternoon.  Other storms will form along an approaching cold front. \nLarge hail and damaging winds will be possible in the strongest\ncells.\n\nThe severe thunderstorm watch area is approximately along and 75\nstatute miles north and south of a line from 50 miles west of Temple\nTX to 40 miles northeast of Shreveport LA. For a complete depiction\nof the watch see the associated watch outline update (WOUS64 KWNS\nWOU5).\n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nREMEMBER...A Severe Thunderstorm Watch means conditions are\nfavorable for severe thunderstorms in and close to the watch area.\nPersons in these areas should be on the lookout for threatening\nweather conditions and listen for later statements and possible\nwarnings. Severe thunderstorms can and occasionally do produce\ntornadoes.\n\n&&\n\nAVIATION...A few severe thunderstorms with hail surface and aloft to\n2.5 inches. Extreme turbulence and surface wind gusts to 60 knots. A\nfew cumulonimbi with maximum tops to 500. Mean storm motion vector\n26030.\n\n...Hart\n\n","title":"SPC issues Tstm Watch 25","valid_ts":null,"warning":null,"watch":{"is_pds":false,"id":25,"watch_type":"SevereThunderstorm","status":"Issued","issued_for":"Southwest Arkansas, Northwest Louisiana, Southeast Oklahoma, Central and Northeast Texas"}}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_svr_watch_cancelled() {
        let product = get_product_from_file("data/products/sel-svr-watch-cancelled");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1556002980000000,"event_type":"NwsSel","expires_ts":null,"ext_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":null,"report":null,"text":"\n000\nWWUS20 KWNS 230703\nSEL4  \nSPC WW 230703\nOKZ000-TXZ000-230700-\n\nURGENT - IMMEDIATE BROADCAST REQUESTED\nSEVERE THUNDERSTORM WATCH - NUMBER 94 \nNWS STORM PREDICTION CENTER NORMAN OK \n203 AM CDT TUE APR 23 2019\n\nTHE NWS STORM PREDICTION CENTER HAS CANCELLED \nSEVERE THUNDERSTORM WATCH NUMBER 94 ISSUED AT 635 PM CDT FOR PORTIONS OF\n\n         OKLAHOMA\n         TEXAS\n\n","title":"SPC cancels Tstm Watch 94","valid_ts":null,"warning":null,"watch":{"is_pds":false,"id":94,"watch_type":"SevereThunderstorm","status":"Cancelled","issued_for":null}}"#;
        assert_eq!(expected, serialized_result);
    }
}
