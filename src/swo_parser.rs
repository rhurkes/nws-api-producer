use super::domain::Product;
use super::parser::{get_parse_error, Regexes};
use wx::domain::{
    Coordinates, Event, EventType, Location, MdConcerning, MesoscaleDiscussion, Outlook,
    OutlookRisk, SwoType,
};
use wx::error::{Error, WxError};
use wx::util;

/**
 * Parses an NWS Severe Storm Outlook Narrative (SWO) product, which includes SPC Outlooks
 * and Mesoscale Discussions (MDs).
 */
pub fn parse(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let swo_type = get_swo_type(&product.product_text);

    match swo_type {
        SwoType::Day1 => parse_outlook(product, swo_type),
        SwoType::Day2 => Ok(None),
        SwoType::Day3 => Ok(None),
        SwoType::Day48 => Ok(None),
        SwoType::MesoscaleDiscussion => parse_md(product, regexes),
        SwoType::Unknown => Err(Error::Wx(<WxError>::new("Unknown SWO type"))),
    }
}

fn get_swo_type(text: &str) -> SwoType {
    if text.contains("ACUS01") {
        SwoType::Day1
    } else if text.contains("ACUS02") {
        SwoType::Day2
    } else if text.contains("ACUS03") {
        SwoType::Day3
    } else if text.contains("ACUS48") {
        SwoType::Day48
    } else if text.contains("ACUS11") {
        SwoType::MesoscaleDiscussion
    } else {
        SwoType::Unknown
    }
}

fn parse_outlook(product: &Product, swo_type: SwoType) -> Result<Option<Event>, Error> {
    let max_risk = get_outlook_risk(&product.product_text);
    let title = format!("SPC {:?} Outlook: {:?}", swo_type, max_risk);
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;

    let outlook = Outlook {
        swo_type,
        max_risk,
        polys: None,
    };

    let event = Event {
        event_ts,
        event_type: EventType::NwsSwo,
        expires_ts: None,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location: None,
        md: None,
        outlook: Some(outlook),
        report: None,
        text: Some(product.product_text.to_string()),
        title,
        valid_ts: None,
        warning: None,
        watch: None,
    };

    Ok(Some(event))
}

fn parse_md(product: &Product, regexes: Regexes) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let id = regexes
        .md_number
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let watch_issuance_probability = regexes.probability.captures(&text);
    let affected = regexes
        .affected
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let wfos = regexes
        .wfos
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;
    let poly = regexes
        .poly
        .captures(&text)
        .ok_or_else(|| get_parse_error(&text))?;

    let poly = build_poly(poly[1].to_string());
    let id = id[1].parse::<u16>()?;
    let watch_issuance_probability = if watch_issuance_probability.is_some() {
        Some(watch_issuance_probability.unwrap()[1].parse::<u16>()?)
    } else {
        None
    };
    let mut concerning = MdConcerning::Unknown;
    let affected = affected[1].to_string().replace('\n', " ");
    let wfos: Vec<String> = wfos[1]
        .split("...")
        .map(ToString::to_string)
        .filter(|s| s != "")
        .collect();

    let title = if text.contains("Concerning...Severe potential...Watch") {
        concerning = MdConcerning::NewSvrWatch;
        format!(
            "SPC MD: Tstm Watch {:?}%",
            watch_issuance_probability.unwrap()
        )
    } else if text.contains("Concerning...Severe potential...Tornado Watch") {
        concerning = MdConcerning::NewTorWatch;
        format!(
            "SPC MD: Tornado Watch {:?}%",
            watch_issuance_probability.unwrap()
        )
    } else if text.contains("Concerning...Severe Thunderstorm Watch") {
        concerning = MdConcerning::ExistingSvrWatch;
        "SPC MD: Existing Tstm Watch".to_string()
    } else if text.contains("Concerning...Tornado Watch") {
        concerning = MdConcerning::ExistingTorWatch;
        "SPC MD: Existing Tornado Watch".to_string()
    } else {
        "SPC Mesoscale Discussion".to_string()
    };

    let md = MesoscaleDiscussion {
        id,
        affected,
        concerning,
        watch_issuance_probability,
        wfos,
    };

    let event_ts = util::ts_to_ticks(&product.issuance_time)?;

    let location = Some(Location {
        wfo: None,
        point: None,
        poly: Some(poly),
    });

    let event = Event {
        event_ts,
        event_type: EventType::NwsSwo,
        expires_ts: None,
        fetch_status: None,
        image_uri: None,
        ingest_ts: 0,
        location,
        md: Some(md),
        outlook: None,
        report: None,
        text: Some(product.product_text.to_string()),
        title,
        valid_ts: None,
        warning: None,
        watch: None,
    };

    Ok(Some(event))
}

/**
 * All risks are listed in a text, so we need to exit early when we find the first
 * match by severity descending.
 */
fn get_outlook_risk(text: &str) -> OutlookRisk {
    if text.contains("THERE IS A HIGH RISK") {
        OutlookRisk::HIGH
    } else if text.contains("THERE IS A MODERATE RISK") {
        OutlookRisk::MDT
    } else if text.contains("THERE IS AN ENHANCED RISK") {
        OutlookRisk::ENH
    } else if text.contains("THERE IS A SLIGHT RISK") {
        OutlookRisk::SLGT
    } else if text.contains("THERE IS A MARGINAL RISK") {
        OutlookRisk::MRGL
    } else {
        OutlookRisk::TSTM
    }
}

pub fn build_poly(watch_poly: String) -> Vec<Coordinates> {
    let watch_poly = watch_poly.replace('\n', "").replace("            ", " ");
    let watch_poly = watch_poly.trim();
    let watch_poly: Vec<Coordinates> = watch_poly
        .split(' ')
        .map(ToString::to_string)
        .map(|s| {
            let lat = s[0..4].parse::<f32>().unwrap() / 100.0;
            let mut lon = s[4..8].parse::<f32>().unwrap();

            if lon < 5000.0 {
                lon += 10000.0;
            }

            lon /= -100.0;

            Coordinates { lat, lon }
        })
        .collect();
    watch_poly
}

#[cfg(test)]
mod tests {
    use super::super::test_util::get_product_from_file;
    use super::*;

    #[test]
    fn parse_swo_md_tor_watch_likely() {
        let product = get_product_from_file("data/products/swo-md-tor-watch-likely");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522773660000000,"event_type":"NwsSwo","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":null,"point":null,"poly":[{"lat":37.82,"lon":-87.69},{"lat":38.53,"lon":-87.76},{"lat":39.73,"lon":-87.06},{"lat":40.62,"lon":-85.25},{"lat":40.46,"lon":-83.56},{"lat":40.36,"lon":-83.1},{"lat":40.12,"lon":-82.74},{"lat":39.65,"lon":-82.75},{"lat":39.24,"lon":-83.39},{"lat":38.8,"lon":-84.23},{"lat":38.2,"lon":-85.03},{"lat":37.81,"lon":-85.97},{"lat":37.73,"lon":-86.3},{"lat":37.64,"lon":-87.21},{"lat":37.82,"lon":-87.69}]},"md":{"id":205,"affected":"Central and southern Indiana...far northern Kentucky...western Ohio","concerning":"NewTorWatch","watch_issuance_probability":95,"wfos":["ILN","LMK","IWX","IND","PAH","ILX"]},"outlook":null,"report":null,"text":"\n504 \nACUS11 KWNS 031641\nSWOMCD\nSPC MCD 031641 \nOHZ000-KYZ000-INZ000-ILZ000-031915-\n\nMesoscale Discussion 0205\nNWS Storm Prediction Center Norman OK\n1141 AM CDT Tue Apr 03 2018\n\nAreas affected...Central and southern Indiana...far northern\nKentucky...western Ohio\n\nConcerning...Severe potential...Tornado Watch likely \n\nValid 031641Z - 031915Z\n\nProbability of Watch Issuance...95 percent\n\nSUMMARY...Storms are expected to increase in intensity this\nafternoon with a few tornadoes possible along with large hail.\nAdditional severe storms are likely later this evening.\n\nDISCUSSION...Scattered storms persist from southern IL across\ncentral IN and into OH along and north of a warm front. This\nboundary will gradually shift northward due to boundary layer\nheating/mixing to the south and strengthening warm air advection via\na backing 50-60 kt low-level jet. While some of the activity is\ncurrently elevated, a transition may occur in a few hours allowing\nstorms along the warm front to become supercells and/or bows.\nAdditional storms may also form south of the warm front as the air\nmass continues to destabilize, most likely across southern IN, far\nnorthern KY, and southwest OH. Wind profiles will become\nincreasingly favorable for supercells and tornadoes throughout the\nday as the low deepens.\n\n..Jewell/Hart.. 04/03/2018\n\n...Please see www.spc.noaa.gov for graphic product...\n\nATTN...WFO...ILN...LMK...IWX...IND...PAH...ILX...\n\nLAT...LON   37828769 38538776 39738706 40628525 40468356 40368310\n            40128274 39658275 39248339 38808423 38208503 37818597\n            37738630 37648721 37828769 \n\n\n","title":"SPC MD: Tornado Watch 95%","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_swo_md_continues() {
        let product = get_product_from_file("data/products/swo-md-continues");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522276380000000,"event_type":"NwsSwo","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":null,"point":null,"poly":[{"lat":33.18,"lon":-90.84},{"lat":34.13,"lon":-90.08},{"lat":34.49,"lon":-89.33},{"lat":34.07,"lon":-88.56},{"lat":32.91,"lon":-89.41},{"lat":32.2,"lon":-90.65},{"lat":31.66,"lon":-91.55},{"lat":31.71,"lon":-91.86},{"lat":32.45,"lon":-91.21},{"lat":33.18,"lon":-100.84}]},"md":{"id":190,"affected":"West central through north central Mississippi and adjacent portions of Arkansas/Louisiana","concerning":"ExistingTorWatch","watch_issuance_probability":null,"wfos":["MEG","JAN"]},"outlook":null,"report":null,"text":"\n205 \nACUS11 KWNS 282233\nSWOMCD\nSPC MCD 282232 \nMSZ000-LAZ000-290030-\n\nMesoscale Discussion 0190\nNWS Storm Prediction Center Norman OK\n0532 PM CDT Wed Mar 28 2018\n\nAreas affected...West central through north central Mississippi and\nadjacent portions of Arkansas/Louisiana\n\nConcerning...Tornado Watch 23...\n\nValid 282232Z - 290030Z\n\nThe severe weather threat for Tornado Watch 23 continues.\n\nSUMMARY...A risk for thunderstorm activity capable of producing\ndamaging wind gusts and a couple of tornadoes will gradually spread\nacross and northeast of the Vicksburg MS area, toward Greenwood and\nTupelo, through 7-9 PM CDT.\n\nDISCUSSION...The risk for severe weather will gradually increase\nacross west central into north central Mississippi through the\n00-02Z time frame.  This will largely occur in association with the\nnortheastward migration of a weak wave along an effective warm\nfrontal zone/zone of enhanced low-level convergence.  Strengthening\nof southerly 850 mb flow to 40-50 kt appears likely to accompany\nthis feature.  This will contribute to enlarging low-level\nhodographs along the boundary, supportive of supercell structures\nwith a risk for potentially damaging wind gusts and perhaps a couple\nof tornadoes.  Northeast of the Vicksburg area, thermodynamic\nprofiles/instability still appears somewhat marginal, but this may\nchange during the next couple of hours with continued low-level\nmoistening.\n\n..Kerr.. 03/28/2018\n\n...Please see www.spc.noaa.gov for graphic product...\n\nATTN...WFO...MEG...JAN...\n\nLAT...LON   33189084 34139008 34498933 34078856 32918941 32209065\n            31669155 31719186 32459121 33180084 \n\n\n","title":"SPC MD: Existing Tornado Watch","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_swo_day1_no_severe() {
        let product = get_product_from_file("data/products/swo-day1-no-severe");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522524900000000,"event_type":"NwsSwo","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":{"swo_type":"Day1","max_risk":"TSTM","polys":null},"report":null,"text":"\n931 \nACUS01 KWNS 311935\nSWODY1\nSPC AC 311934\n\nDay 1 Convective Outlook  \nNWS Storm Prediction Center Norman OK\n0234 PM CDT Sat Mar 31 2018\n\nValid 312000Z - 011200Z\n\n...NO SEVERE THUNDERSTORM AREAS FORECAST...\n\n...SUMMARY...\nThunderstorms are possible from southern Oklahoma across the Ozarks\nregion and over parts of the Florida Peninsula.\n\n...Discussion...\n\nNo changes to 1630z outlook are warranted.\n\n..Darrow.. 03/31/2018\n\n.PREV DISCUSSION... /ISSUED 1126 AM CDT Sat Mar 31 2018/\n\n...TX/OK into the Ozarks...\nA strong surface cold front is surging southward across KS, and will\nmove into parts of TX/OK/AR/MO later this evening.  Southerly\nlow-level winds ahead of the front will continue to moisten the\nregion, leading to a corridor of marginal CAPE values by late\nafternoon.  Virtually all 12z model guidance is consistent in the\ndevelopment of scattered showers and a few thunderstorms along/ahead\nof the front later today.  Shear profiles would be conditionally\nconducive for organized/rotating updrafts.  However, weak low-level\nconvergence/shear and some weak capping inversion are expected to\nlimit updraft strength and resultant severe risk.  One or two cells\nmay briefly approach severe limits producing hail, but the overall\nrisk appears to warrant a continuation of less-than-5% hail\nprobabilities at this time.\n\n$$\n\n","title":"SPC Day1 Outlook: TSTM","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_swo_day1_moderate() {
        let product = get_product_from_file("data/products/swo-day1-moderate");
        let regexes = Regexes::new();
        let result = parse(&product, regexes).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522777200000000,"event_type":"NwsSwo","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":null,"md":null,"outlook":{"swo_type":"Day1","max_risk":"MDT","polys":null},"report":null,"text":"\n587 \nACUS01 KWNS 031740\nSWODY1\nSPC AC 031739\n\nDay 1 Convective Outlook CORR 1\nNWS Storm Prediction Center Norman OK\n1239 PM CDT Tue Apr 03 2018\n\nValid 031630Z - 041200Z\n\n...THERE IS A MODERATE RISK OF SEVERE THUNDERSTORMS OVER PARTS OF\nEASTERN ARKANSAS...NORTHERN MISSISSIPPI...WESTERN AND MIDDLE\nTENNESSEE...SOUTHEAST MISSOURI...SOUTHERN ILLINOIS...WESTERN AND\nCENTRAL KENTUCKY...SOUTHERN AND CENTRAL INDIANA...AND SOUTHWEST\nOHIO...\n\n...THERE IS AN ENHANCED RISK OF SEVERE THUNDERSTORMS SURROUNDING THE\nMODERATE RISK AREA OVER PARTS OF THE LOWER AND MID\nMISSISSIPPI...OHIO...AND TENNESSEE VALLEYS...\n\n...THERE IS A SLIGHT RISK OF SEVERE THUNDERSTORMS FROM CENTRAL TEXAS\nINTO OHIO...\n\n...THERE IS A MARGINAL RISK OF SEVERE THUNDERSTORMS FROM CENTRAL\nTEXAS INTO WESTERN PENNSYLVANIA...\n\nCORRECTED SMALL TEXT ERROR\n\n...SUMMARY...\nA Moderate Risk for thunderstorms producing widespread damaging\nwinds, large hail, and a few tornadoes exists over parts of the Ohio\nValley and Mid-South regions.\n\n...AR/MS northeastward through much of the OH/TN Valleys...\nAn active severe weather day is expected across the MS/OH/TN Valleys\ntoday with numerous strong/severe thunderstorms affecting a large\narea.  The primary focus for severe storms will be a progressive and\ndeepening shortwave trough moving across the central Plains.  A\ndeepening surface low and cold front in advance of this system will\nsweep across the risk area this afternoon through tonight, resulting\nin a fast-moving squall line extending from IL/IN/OH southward into\nthe Mid South.  Visible satellite imagery shows broken cloud cover\nacross most of the warm sector, promoting heating and\ndestabilization.  Forecast soundings suggest a corridor of moderate\nCAPE values ahead of the front by mid-afternoon as thunderstorms\nbegin to form.  Initial activity may be supercellular in nature,\nwith a risk of tornadoes (some strong) and large hail from northeast\nAR/western MS into parts of southern IL/IN and western KY. \nEventually, the storms should congeal into a line with multiple\nbowing segments as it progresses across the MDT and ENH risk areas\nwith the potential for widespread damaging winds and a few QLCS\ntornadoes.\n\n...TX/LA...\nScattered strong to severe thunderstorms have developed this morning\nover central TX, ahead of a southern stream shortwave trough.  This\nactivity will persist through the day and spread into LA, with a\nrisk of large hail and damaging wind gusts.  By mid-afternoon,\nthunderstorms are expected to form along the advancing cold front\nand affect these same areas.\n\n...IN/OH...\nA persistent cluster of thunderstorms is affecting much of\ncentral/northern IN and OH.  The air mass south of the activity\ncontinues to warm, leading to a favorable environment for\nsevere/supercell thunderstorms.  This corridor remains in the higher\ntornado/damaging wind probability area for both warm frontal\nactivity this afternoon, and the squall line activity later today.\n\n..Hart.. 04/03/2018\n\n$$\n\n","title":"SPC Day1 Outlook: MDT","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_swo_day2_no_severe() {
        let product = get_product_from_file("data/products/swo-day2-no-severe");
        let regexes = Regexes::new();
        let result = parse(&product, regexes);
        assert!(result.is_err());
    }

    #[test]
    fn build_poly_with_spaces_and_breaks() {
        let input = " 40468356 40368310\n            40128274 ".to_string();
        let result = build_poly(input);
        let expected = vec![
            Coordinates {
                lat: 40.46,
                lon: -83.56,
            },
            Coordinates {
                lat: 40.36,
                lon: -83.1,
            },
            Coordinates {
                lat: 40.12,
                lon: -82.74,
            },
        ];
        assert_eq!(expected, result);
    }

    #[test]
    fn build_poly_with_lon_greater_than_100() {
        let input = "40468356 40368310 40120274".to_string();
        let result = build_poly(input);
        let expected = vec![
            Coordinates {
                lat: 40.46,
                lon: -83.56,
            },
            Coordinates {
                lat: 40.36,
                lon: -83.1,
            },
            Coordinates {
                lat: 40.12,
                lon: -102.74,
            },
        ];
        assert_eq!(expected, result);
    }
}
