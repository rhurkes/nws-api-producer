use super::domain::Product;
use wx::domain::{Event, EventType, Location};
use wx::error::Error;
use wx::util;

/**
 * Parses an NWS Severe Weather Statement (SWS). Primarily used in this API for Tornado Emergency
 * or Particularly Dangerous Situation attribution.
 */
pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let lower_case_text = text.to_lowercase();
    let is_tor_emergency = lower_case_text.contains("tornado emergency");
    let is_pds = lower_case_text.contains("particularly dangerous situation");

    if !is_tor_emergency && !is_pds {
        return Ok(None);
    }

    let title_fragment = if is_tor_emergency {
        if is_pds {
            "PDS Tor Emergency"
        } else {
            "Tornado Emergency"
        }
    } else {
        "PDS Tornado"
    };
    let wfo = product.issuing_office.to_string();
    let title = format!("{} SVS: {}", wfo, title_fragment); // 31 chars max
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;

    let location = Some(Location {
        point: None,
        poly: None,
        wfo: Some(wfo),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsSvs,
        expires_ts: None,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location,
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
    use super::super::test_util::get_product_from_file;
    use super::*;

    #[test]
    fn parse_svs_product_nothing_interesting() {
        let product = get_product_from_file("data/products/svs-tor");
        let result = parse(&product).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_svs_product_pds() {
        let product = get_product_from_file("data/products/svs-pds-tor");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525223280000000,"event_type":"NwsSvs","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":null,"poly":null},"md":null,"outlook":null,"report":null,"text":"\n211 \nWWUS53 KTOP 020108\nSVSTOP\n\nSevere Weather Statement\nNational Weather Service Topeka KS\n808 PM CDT TUE MAY 1 2018\n\nKSC143-020130-\n/O.CON.KTOP.TO.W.0008.000000T0000Z-180502T0130Z/\nOttawa-\n808 PM CDT TUE MAY 1 2018\n\n...A TORNADO WARNING REMAINS IN EFFECT UNTIL 830 PM CDT FOR\nSOUTHEASTERN OTTAWA COUNTY...\n    \nAt 807 PM CDT, a confirmed extremely dangerous tornado was located 4 \nmiles south of Minneapolis, moving northeast at 30 mph. An \nadditional tornado may be forming 5 miles NW of Bennington.\n\nThis is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW!\n\nHAZARD...Damaging tornado. \n\nSOURCE...Law enforcement confirmed tornado. \n\nIMPACT...You are in a life-threatening situation. Flying debris may \n         be deadly to those caught without shelter. Mobile homes \n         will be destroyed. Considerable damage to homes, \n         businesses, and vehicles is likely and complete destruction \n         is possible. \n\nThe tornado will be near...\n  Bennington around 815 PM CDT. \n  Wells around 825 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nHeavy rainfall may hide this tornado. Do not wait to see or hear the\ntornado. TAKE COVER NOW!\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3926 9748 3897 9738 3897 9773 3910 9783\nTIME...MOT...LOC 0107Z 244DEG 27KT 3906 9769 \n\nTORNADO...OBSERVED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nSkow\n\n","title":"KTOP SVS: PDS Tornado","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_svs_product_tornado_emergency() {
        let product = get_product_from_file("data/products/svs-tor-emergency");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525223280000000,"event_type":"NwsSvs","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":null,"poly":null},"md":null,"outlook":null,"report":null,"text":"\n211 \nWWUS53 KTOP 020108\nSVSTOP\n\nSevere Weather Statement\nNational Weather Service Topeka KS\n808 PM CDT TUE MAY 1 2018\n\nKSC143-020130-\n/O.CON.KTOP.TO.W.0008.000000T0000Z-180502T0130Z/\nOttawa-\n808 PM CDT TUE MAY 1 2018\n\n...TORNADO EMERGENCY IN TOPEKA METRO AREA...\n    \nAt 807 PM CDT, a confirmed extremely dangerous tornado was located 4 \nmiles south of Minneapolis, moving northeast at 30 mph. An \nadditional tornado may be forming 5 miles NW of Bennington.\n\n TAKE COVER NOW!\n\nHAZARD...Damaging tornado. \n\nSOURCE...Law enforcement confirmed tornado. \n\nIMPACT...You are in a life-threatening situation. Flying debris may \n         be deadly to those caught without shelter. Mobile homes \n         will be destroyed. Considerable damage to homes, \n         businesses, and vehicles is likely and complete destruction \n         is possible. \n\nThe tornado will be near...\n  Bennington around 815 PM CDT. \n  Wells around 825 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nHeavy rainfall may hide this tornado. Do not wait to see or hear the\ntornado. TAKE COVER NOW!\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3926 9748 3897 9738 3897 9773 3910 9783\nTIME...MOT...LOC 0107Z 244DEG 27KT 3906 9769 \n\nTORNADO...OBSERVED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nSkow\n\n","title":"KTOP SVS: Tornado Emergency","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_svs_product_tornado_emergency_and_pds() {
        let product = get_product_from_file("data/products/svs-pds-tor-emergency");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525223280000000,"event_type":"NwsSvs","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":null,"poly":null},"md":null,"outlook":null,"report":null,"text":"\n211 \nWWUS53 KTOP 020108\nSVSTOP\n\nSevere Weather Statement\nNational Weather Service Topeka KS\n808 PM CDT TUE MAY 1 2018\n\nKSC143-020130-\n/O.CON.KTOP.TO.W.0008.000000T0000Z-180502T0130Z/\nOttawa-\n808 PM CDT TUE MAY 1 2018\n\n...A TORNADO EMERGENCY REMAINS IN EFFECT UNTIL 830 PM CDT FOR\nSOUTHEASTERN OTTAWA COUNTY...\n    \nAt 807 PM CDT, a confirmed extremely dangerous tornado was located 4 \nmiles south of Minneapolis, moving northeast at 30 mph. An \nadditional tornado may be forming 5 miles NW of Bennington.\n\nThis is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW!\n\nHAZARD...Damaging tornado. \n\nSOURCE...Law enforcement confirmed tornado. \n\nIMPACT...You are in a life-threatening situation. Flying debris may \n         be deadly to those caught without shelter. Mobile homes \n         will be destroyed. Considerable damage to homes, \n         businesses, and vehicles is likely and complete destruction \n         is possible. \n\nThe tornado will be near...\n  Bennington around 815 PM CDT. \n  Wells around 825 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nHeavy rainfall may hide this tornado. Do not wait to see or hear the\ntornado. TAKE COVER NOW!\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3926 9748 3897 9738 3897 9773 3910 9783\nTIME...MOT...LOC 0107Z 244DEG 27KT 3906 9769 \n\nTORNADO...OBSERVED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nSkow\n\n","title":"KTOP SVS: Tor Emergency, PDS","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }
}
