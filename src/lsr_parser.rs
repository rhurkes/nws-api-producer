use super::domain::Product;
use chrono::prelude::*;
use wx::domain::{Coordinates, Event, EventType, HazardType, Location, Report, Units};
use wx::error::{Error, WxError};
use wx::util;

const AGE_THRESHOLD_MICROS: u64 = 60 * 60 * 1000 * 1000;

// Intermediary structure for an LSR to make parsing easier
#[derive(Debug)]
struct Skeleton<'a> {
    top_line: &'a str,
    bottom_line: &'a str,
    lines: Vec<&'a str>,
    remarks_index: usize,
    end_index: usize,
}

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let text = &product.product_text;
    let skeleton = get_skeleton(&text)?;

    if skeleton.is_none() {
        return Ok(None);
    }

    let lsr = skeleton.unwrap();
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let raw_ts = lsr.bottom_line.get(0..10).unwrap().to_string() + lsr.top_line.get(0..7).unwrap();
    let offset: Vec<&str> = lsr.lines[7].split(' ').collect();
    let offset = util::tz_to_offset(offset[2])?;
    let raw_ts = raw_ts + offset;
    let report_ts = get_report_ticks(&raw_ts)?;

    // Skip reports too far in the past, especially since these can come hours, days, or even months later
    if event_ts - report_ts > AGE_THRESHOLD_MICROS {
        return Ok(None);
    }

    let raw_point = lsr.top_line.get(53..).unwrap().replace("W", "");
    let raw_point = raw_point.trim();
    let lon: f32 = raw_point.get(7..).unwrap().trim().parse().unwrap();
    let lon = lon * -1.0;
    let point = Some(Coordinates {
        lat: raw_point.get(0..5).unwrap().parse()?,
        lon,
    });

    let wfo = &product.issuing_office;
    let raw_hazard = lsr.top_line.get(12..29).unwrap().trim();
    let hazard = get_lsr_hazard_type(raw_hazard);
    let mut was_measured = None;
    let mut units = None;
    let mut magnitude = None;
    let raw_mag = lsr.bottom_line.get(12..29).unwrap().trim();
    let county = lsr.bottom_line.get(29..48).unwrap().trim().to_string();
    let mut title = "Report: ".to_string();

    if !raw_mag.is_empty() {
        was_measured = Some(raw_mag.get(0..1).unwrap() == "M");
        let space_index = raw_mag.find(' ').unwrap();
        if raw_mag.contains("MPH") {
            units = Some(Units::Mph);
            magnitude = Some(raw_mag.get(1..space_index).unwrap().parse().unwrap());
            title = format!("{} {}mph", title, magnitude.unwrap());
        } else if raw_mag.contains("INCH") {
            units = Some(Units::Inches);
            magnitude = Some(raw_mag.get(1..space_index).unwrap().parse().unwrap());
            title = format!("{} {}\"", title, magnitude.unwrap());
        }
    }

    let title = format!("{} {:?} ({})", title, hazard, wfo);

    let location = Location {
        point,
        poly: None,
        wfo: Some(wfo.to_string()),
        county: Some(county),
    };

    // CO-OP OBSERVER, TRAINED SPOTTER, STORM CHASER, PUBLIC, EMERGENCY MNGR, ASOS, AWOS,
    // NWS EMPLOYEE, OFFICIAL NWS OBS, NWS STORM SURVEY, AMATEUR RADIO, BROADCAST MEDIA, etc.
    let reporter = lsr.bottom_line.get(53..).unwrap().trim().to_string();

    let report = Report {
        hazard,
        magnitude,
        report_ts: Some(report_ts),
        reporter,
        units,
        was_measured,
    };

    let mut event = Event::new(event_ts, EventType::NwsLsr, title);
    event.location = Some(location);
    event.report = Some(report);
    event.text = Some(text.to_string());

    Ok(Some(event))
}

fn get_skeleton(text: &str) -> Result<Option<Skeleton>, Error> {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() < 16 {
        return Err(Error::Wx(<WxError>::new("invalid LSR body: too few lines")));
    }

    if lines[5].contains("SUMMARY") {
        return Ok(None);
    }

    if lines[5].contains("CORRECTED") {
        return Ok(None);
    }

    let mut remarks_index = None;
    let mut end_index = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("..REMARKS..") {
            remarks_index = Some(i);
        }

        // This delimiter doesn't always appear...
        if line.contains("&&") {
            end_index = Some(i);
        }

        // ...but this one should...
        if line.contains("$$") && end_index.is_none() {
            end_index = Some(i);
        }
    }

    // ...and if it doesn't we really don't want things to blow up.
    if end_index.is_none() {
        end_index = Some(lines.len() - 1);
    }

    if remarks_index.is_none() {
        return Err(Error::Wx(<WxError>::new(
            "invalid LSR body: missing REMARKS",
        )));
    }

    let remarks_index = remarks_index.unwrap();
    let end_index = end_index.unwrap();
    let top_line = lines[remarks_index + 2];
    let bottom_line = lines[remarks_index + 3];
    if top_line.len() < 53 || bottom_line.len() < 53 {
        return Err(Error::Wx(<WxError>::new(
            "invalid LSR body: missing details",
        )));
    }

    Ok(Some(Skeleton {
        bottom_line,
        top_line,
        remarks_index,
        end_index,
        lines,
    }))
}

fn get_lsr_hazard_type(input: &str) -> HazardType {
    match input {
        "TORNADO" => HazardType::Tornado,
        "HAIL" => HazardType::Hail,
        "FLOOD" => HazardType::Flood,
        "FREEZING RAIN" => HazardType::FreezingRain,
        "TSTM WND GST" => HazardType::Wind,
        "TSTM WND DMG" => HazardType::Wind,
        "NON-TSTM WND GST" => HazardType::Wind,
        "NON-TSTM WND DMG" => HazardType::Wind,
        "MARINE TSTM WIND" => HazardType::Wind,
        _ => HazardType::Other,
    }
}

fn get_report_ticks(input: &str) -> Result<u64, Error> {
    Ok(DateTime::parse_from_str(input, "%m/%d/%Y%I%M %p%z")?.timestamp_millis() as u64 * 1000)
}

#[cfg(test)]
mod tests {
    use super::super::test_util::get_product_from_file;
    use super::*;

    #[test]
    fn get_report_ticks_should_return_correct_ticks() {
        let time = "03/13/20190300 PM+0400";
        let result = get_report_ticks(time).unwrap();
        assert_eq!(result, 1552474800000000);
    }

    #[test]
    fn get_skeleton_too_few_lines_should_be_an_error() {
        let text = "\n\n\n\n\n\n\n\n\n\n\n\nthis is bad text";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_summary_should_be_an_ok_none() {
        let product = get_product_from_file("data/products/lsr-summary");
        let result = get_skeleton(&product.product_text).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_skeleton_corrected_should_be_an_error() {
        let product = get_product_from_file("data/products/lsr-corrected-tstm-wind-dmg");
        let result = get_skeleton(&product.product_text).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn get_skeleton_no_remarks_index_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            \n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_double_and_should_be_handled() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_ok());
    }

    #[test]
    fn get_skeleton_no_end_index_should_be_handled() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_ok());
    }

    #[test]
    fn get_skeleton_no_top_details_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_bottom_details_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n\n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tornado_report() {
        let product = get_product_from_file("data/products/lsr-tornado");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522524900000000,"event_type":"NwsLsr","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KMFL","point":{"lat":26.8,"lon":-80.64},"poly":null,"county":"PALM BEACH"},"md":null,"outlook":null,"report":{"reporter":"TRAINED SPOTTER","hazard":"Tornado","magnitude":null,"units":null,"was_measured":null,"report_ts":1522522800000000},"text":"\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE MIAMI FL\n335 PM EDT SAT MAR 31 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0300 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n03/31/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\nEVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n","title":"Report:  Tornado (KMFL)","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_old_report_should_be_ok_none() {
        let product = get_product_from_file("data/products/lsr-tornado-old");
        let result = parse(&product).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_wind_speed_report() {
        let product = get_product_from_file("data/products/lsr-tstm-wind");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1555316100000000,"event_type":"NwsLsr","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KMHX","point":{"lat":35.07,"lon":-77.04},"poly":null,"county":"CRAVEN"},"md":null,"outlook":null,"report":{"reporter":"ASOS","hazard":"Wind","magnitude":61.0,"units":"Mph","was_measured":true,"report_ts":1555315080000000},"text":"\n000\nNWUS52 KMHX 150815\nLSRMHX\n\nPRELIMINARY LOCAL STORM REPORT\nNATIONAL WEATHER SERVICE NEWPORT/MOREHEAD CITY NC\n415 AM EDT MON APR 15 2019\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0358 AM     TSTM WND GST     COASTAL CAROLINA REGION 35.07N 77.04W\n04/15/2019  M61 MPH          CRAVEN             NC   ASOS             \n\n            NEW BERN/CRAVEN COUNTY ASOS (EWN) REPORTS \n            GUST OF 61 MPH. \n\n\n&&\n\n$$\n\nML\n\n","title":"Report:  61mph Wind (KMHX)","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_hail_report() {
        let product = get_product_from_file("data/products/lsr-hail-remarks");
        let result = parse(&product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522113360000000,"event_type":"NwsLsr","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KSJT","point":{"lat":32.07,"lon":-100.66},"poly":null,"county":"COKE"},"md":null,"outlook":null,"report":{"reporter":"STORM CHASER","hazard":"Hail","magnitude":1.25,"units":"Inches","was_measured":false,"report_ts":1522112100000000},"text":"\n106 \nNWUS54 KSJT 270116\nLSRSJT\n\nPRELIMINARY LOCAL STORM REPORT\nNational Weather Service San Angelo Tx\n816 PM CDT MON MAR 26 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0755 PM     HAIL             1 E SILVER              32.07N 100.66W\n03/26/2018  E1.25 INCH       COKE               TX   STORM CHASER    \n\n            1.25 HAIL ON HWY 208 NEAR SILVER \n\n\n&&\n\nEVENT NUMBER SJT1800032\n\n$$\n\nSJT\n\n","title":"Report:  1.25\" Hail (KSJT)","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(expected, serialized_result);
    }

    #[test]
    fn parse_should_not_handle_multiple_events() {
        let product = get_product_from_file("data/products/lsr-multiple-heavy-rain");
        let result = parse(&product).unwrap();
        assert!(result.is_none());
    }
}
