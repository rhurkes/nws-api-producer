use super::domain::Product;
use chrono::prelude::*;
use wx::domain::{Coordinates, Event, EventType, HazardType, Location, Report, Units};
use wx::error::{Error, WxError};
use wx::util;

// TODO do I want to handle corrected LSRs?
// TODO implement all lsr types

const AGE_THRESHOLD_MICROS: u64 = 60 * 60 * 1000 * 1000;

// An intermediary structure for an LSR to make parsing easier
struct Skeleton<'a> {
    top_line: &'a str,
    bottom_line: &'a str,
    lines: Vec<&'a str>,
    remarks_index: usize,
    end_index: usize,
}

pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let lsr = get_skeleton(&product.product_text)?;
    let event_ts = util::ts_to_ticks(&product.issuance_time)?;
    let raw_ts = lsr.bottom_line.get(0..10).unwrap().to_string() + lsr.top_line.get(0..7).unwrap();
    let offset: Vec<&str> = lsr.lines[7].split(' ').collect(); // is this always the date line? TODO
    let offset = util::tz_to_offset(offset[2])?;
    let raw_ts = raw_ts + offset;
    let report_ts = get_report_ticks(&raw_ts)?;

    // Skip reports too far in the past, especially since these can come hours, days, or even months later
    if event_ts - report_ts > AGE_THRESHOLD_MICROS {
        return Ok(None);
    }

    if report_ts > event_ts {
        // TODO log warning - report ts should be before event ts, but strange things happen
    }

    let raw_point = lsr.top_line.get(53..).unwrap().replace("W", "");
    let raw_point = raw_point.trim();
    let lon: f32 = raw_point.get(7..).unwrap().trim().parse().unwrap();
    let lon = lon * -1.0;
    let point = Some(Coordinates {
        lat: raw_point.get(0..5).unwrap().parse()?,
        lon,
    });

    // Summary can span multiple lines and will have 12 consecutive spaces embedded
    let mut text = "".to_string();
    for i in (lsr.remarks_index + 5)..lsr.end_index {
        text += lsr.lines[i as usize];
    }
    let text = text.replace("            ", "").trim().to_string();

    let wfo = &product.issuing_office;
    let raw_hazard = lsr.top_line.get(12..29).unwrap().trim();
    let hazard = get_lsr_hazard_type(raw_hazard);
    let mut was_measured = None;
    let mut units = None;
    let mut magnitude = None;
    let raw_mag = lsr.bottom_line.get(12..29).unwrap().trim();
    let mut title = wfo.to_string() + " reports";

    if !raw_mag.is_empty() {
        was_measured = Some(raw_mag.get(0..1).unwrap() == "M");
        let space_index = raw_mag.find(' ').unwrap();
        if raw_mag.contains("MPH") {
            units = Some(Units::Mph);
            magnitude = Some(raw_mag.get(1..space_index).unwrap().parse().unwrap());
            title = format!("{} {} MPH", title, magnitude.unwrap());
        } else if raw_mag.contains("INCH") {
            units = Some(Units::Inches);
            magnitude = Some(raw_mag.get(1..space_index).unwrap().parse().unwrap());
            title = format!("{} {} INCH", title, magnitude.unwrap());
        }
    }

    let title = format!("{} {:?}", title, hazard);

    let location = Location {
        point,
        poly: None,
        wfo: Some(wfo.to_string()),
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
    event.text = Some(text);

    Ok(Some(event))
}

fn get_skeleton(text: &str) -> Result<Skeleton, Error> {
    let lines: Vec<&str> = text.lines().collect();

    if lines.len() < 16 {
        return Err(Error::Wx(<WxError>::new("invalid LSR body: too few lines")));
    }

    if lines[5].contains("SUMMARY") {
        return Err(Error::Wx(<WxError>::new("summary LSR should be skipped")));
    }

    let mut remarks_index = None;
    let mut end_index = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("..REMARKS..") {
            remarks_index = Some(i);
        }
        if line.contains("&&") {
            end_index = Some(i);
        }
    }

    if remarks_index.is_none() {
        return Err(Error::Wx(<WxError>::new(
            "invalid LSR body: missing REMARKS",
        )));
    }
    if end_index.is_none() {
        return Err(Error::Wx(<WxError>::new("invalid LSR body: missing &&")));
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

    Ok(Skeleton {
        bottom_line,
        top_line,
        remarks_index,
        end_index,
        lines,
    })
}

fn get_lsr_hazard_type(input: &str) -> HazardType {
    match input {
        "TORNADO" => HazardType::Tornado,
        "HAIL" => HazardType::Hail,
        "FLOOD" => HazardType::Flood,
        "FREEZING RAIN" => HazardType::FreezingRain,
        _ => HazardType::Other {
            kind: input.to_string(),
        },
    }
}

fn lsr_time_to_ticks(input: &str) -> Result<u64, Error> {
    Ok(DateTime::parse_from_str(input, "%I%M %p %z %a %b %d %Y")?.timestamp_millis() as u64 * 1000)
}

fn get_report_ticks(input: &str) -> Result<u64, Error> {
    Ok(DateTime::parse_from_str(input, "%m/%d/%Y%I%M %p%z")?.timestamp_millis() as u64 * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lsr_time_to_ticks_should_return_correct_ticks() {
        let lsr_time = "1008 PM +0600 WED MAR 06 2019";
        let result = lsr_time_to_ticks(lsr_time).unwrap();
        assert_eq!(result, 1551888480000000);
    }

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
    fn get_skeleton_summary_should_be_an_error() {
        let text = "\n523 \nNWUS55 KBOU 271240\nLSRBOU\n\nPRELIMINARY LOCAL STORM REPORT...SUMMARY\nNATIONAL WEATHER SERVICE DENVER CO\n640 AM MDT TUE MAR 27 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0637 AM     SNOW             2 N ARVADA              39.85N 105.09W\n03/27/2018  M2.5 INCH        JEFFERSON          CO   PUBLIC           \n\n            STORM TOTAL. 0.34 INCHES OF WATER \n            EQUIVALENT. REPORT VIA SOCIAL MEDIA. \n\n0635 AM     HEAVY SNOW       ROXBOROUGH PARK         39.44N 105.07W\n03/27/2018  M7.0 INCH        DOUGLAS            CO   PUBLIC           \n\n            STORM TOTAL. REPORT VIA SOCIAL MEDIA. \n\n0634 AM     HEAVY SNOW       HIGHLANDS RANCH         39.55N 104.96W\n03/27/2018  M7.0 INCH        DOUGLAS            CO   TRAINED SPOTTER  \n\n            STORM TOTAL. \n\n0557 AM     SNOW             2 W LEYDEN              39.82N 105.16W\n03/27/2018  M4.0 INCH        JEFFERSON          CO   PUBLIC           \n\n            STORM TOTAL. \n\n0556 AM     SNOW             2 NE THORNTON           39.92N 104.93W\n03/27/2018  M1.9 INCH        ADAMS              CO   PUBLIC           \n\n            STORM TOTAL. \n\n0549 AM     SNOW             3 SSW BEVERLY HILLS     39.43N 104.90W\n03/27/2018  M5.8 INCH        DOUGLAS            CO   TRAINED SPOTTER  \n\n            STORM TOTAL. \n\n0547 AM     HEAVY SNOW       1 N GENESEE             39.70N 105.28W\n03/27/2018  M7.5 INCH        JEFFERSON          CO   PUBLIC           \n\n            STORM TOTAL AS OF 0540. LIGHT SNOW \n            CONTINUES. \n\n0511 AM     HEAVY SNOW       2 NW SMOKY HILL         39.63N 104.78W\n03/27/2018  M5.8 INCH        ARAPAHOE           CO   PUBLIC           \n\n            STORM TOTAL. HEAVY WET SNOW. NO DRIFTING. \n\n0334 AM     HEAVY SNOW       2 SW ARAPAHOE PARK      39.61N 104.71W\n03/27/2018  M8.2 INCH        ARAPAHOE           CO   PUBLIC           \n\n            STORM TOTAL. \n\n0154 AM     SNOW             1 NW WELBY              39.85N 104.98W\n03/27/2018  M2.0 INCH        ADAMS              CO   PUBLIC           \n\n            STORM TOTAL. \n\n1015 PM     SNOW             1 SE BERGEN PARK        39.68N 105.35W\n03/26/2018  M5.5 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0140 AM     HEAVY SNOW       3 W JAMESTOWN           40.11N 105.44W\n03/27/2018  M4.8 INCH        BOULDER            CO   TRAINED SPOTTER  \n\n            SNOW FELL FROM LATE AFTERNOON ONWARDS, NEVER \n            REACHING 1&QUOT;/H. STILL MELTING LAST \n            INCREMENT, BUT ESTIMATING 0.4&QUOT; LIQUID \n            TOTAL OR JUST BELOW THAT. TEMPERATURE DOWN \n            TO 20F (IT NEVER RAINED DURING THIS STORM). \n\n1203 AM     HEAVY SNOW       3 N CONIFER             39.57N 105.31W\n03/27/2018  M7.5 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n            CURRENTLY LIGHT SNOWFALL, TEMP = 25F, \n            STEADY, CALM ... STORM TOTAL SNOWFALL SO FAR \n            = 7.5&QUOT;. \n\n1200 AM     HEAVY SNOW       1 NE ECHO LAKE          39.66N 105.59W\n03/27/2018  E11.0 INCH       CLEAR CREEK        CO   MESONET          \n\n            ECHO LAKE SNOTEL. \n\n1200 AM     SNOW             3 SW WARD               40.04N 105.54W\n03/27/2018  E5.0 INCH        BOULDER            CO   MESONET          \n\n            NIWOT SNOTEL. \n\n1200 AM     SNOW             3 SSW BOULDER           39.99N 105.26W\n03/27/2018  M2.7 INCH        BOULDER            CO   NWS EMPLOYEE     \n\n            NWS OFFICE. \n\n1125 PM     HEAVY SNOW       2 W LOUISVILLE          39.96N 105.18W\n03/26/2018  M4.5 INCH        BOULDER            CO   TRAINED SPOTTER  \n\n            4.5 INCHES MEASURED ATOP MAIL BOXES AND ON \n            GRASSY SURFACES -- SNOW IS VERY HEAVY AND \n            WET. SNOW BEGAN AROUND 7 PM. CURRENT \n            WEATHER: LIGHT SNOW. \n\n1100 PM     SNOW             WINTER PARK             39.89N 105.78W\n03/26/2018  E3.0 INCH        GRAND              CO   PUBLIC           \n\n             \n\n1100 PM     SNOW             LONE TREE               39.54N 104.89W\n03/26/2018  M4.2 INCH        DOUGLAS            CO   PUBLIC           \n\n             \n\n1100 PM     SNOW             BRECKENRIDGE            39.51N 106.05W\n03/26/2018  E2.0 INCH        SUMMIT             CO   PUBLIC           \n\n             \n\n1045 PM     SNOW             LAFAYETTE               39.99N 105.10W\n03/26/2018  E2.5 INCH        BOULDER            CO   PUBLIC           \n\n             \n\n1020 PM     HEAVY SNOW       4 SE PINECLIFFE         39.88N 105.39W\n03/26/2018  M10.2 INCH       JEFFERSON          CO   TRAINED SPOTTER  \n\n            24 HOUR TOTAL, STORM TOTAL SO FAR. \n\n1016 PM     SNOW             2 SW ROCKY FLATS        39.87N 105.23W\n03/26/2018  M4.5 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0100 AM     SNOW             DENVER INTL AIRPORT     39.87N 104.67W\n03/27/2018  M1.5 INCH        DENVER             CO   OFFICIAL NWS OBS \n\n             \n\n1012 PM     SNOW             4 SW BEVERLY HILLS      39.43N 104.90W\n03/26/2018  M1.7 INCH        DOUGLAS            CO   TRAINED SPOTTER  \n\n             \n\n1001 PM     SNOW             4 NE NEDERLAND          39.99N 105.45W\n03/26/2018  M4.5 INCH        BOULDER            CO   TRAINED SPOTTER  \n\n             \n\n1000 PM     SNOW             1 NNW CAMERON PASS      40.53N 105.89W\n03/26/2018  E3.0 INCH        LARIMER            CO   MESONET          \n\n            JOE WRIGHT SNOTEL. \n\n0945 PM     HEAVY SNOW       2 NW FLOYD HILL         39.74N 105.45W\n03/26/2018  M9.0 INCH        CLEAR CREEK        CO   TRAINED SPOTTER  \n\n             \n\n0945 PM     HEAVY SNOW       2 NW FLOYD HILL         39.74N 105.45W\n03/26/2018  M9.0 INCH        CLEAR CREEK        CO   TRAINED SPOTTER  \n\n             \n\n0937 PM     HEAVY SNOW       PINECLIFFE              39.93N 105.43W\n03/26/2018  M7.0 INCH        GILPIN             CO   BROADCAST MEDIA  \n\n             \n\n0931 PM     HEAVY SNOW       1 N GENESEE             39.70N 105.28W\n03/26/2018  M4.3 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0931 PM     SNOW             1 N GENESEE             39.70N 105.28W\n03/26/2018  M4.3 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0924 PM     SNOW             3 N CONIFER             39.57N 105.31W\n03/26/2018  M4.1 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0916 PM     HEAVY SNOW       1 S CRESCENT VILLAGE    39.91N 105.34W\n03/26/2018  M8.0 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n             \n\n0901 PM     HEAVY SNOW       4 SE PINECLIFFE         39.88N 105.39W\n03/26/2018  M6.0 INCH        JEFFERSON          CO   TRAINED SPOTTER  \n\n            6IN WITH 0.51IN SWE SINCE 5:30PM. HEAVY SNOW \n            CONTINUES. \n\n0800 PM     SNOW             2 SSW BOULDER           40.00N 105.27W\n03/26/2018  M0.9 INCH        BOULDER            CO   NWS EMPLOYEE     \n\n            DAVID SKAGGS RESEARCH CENTER NOAA. \n\n\n&&\n\n$$\n\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_remarks_index_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            \n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_end_index_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_top_details_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn get_skeleton_no_bottom_details_should_be_an_error() {
        let text = "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n\n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n";
        let result = get_skeleton(text);
        assert!(result.is_err());
    }

    #[test]
    fn parse_tornado_report() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T00:08:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "LSR".to_string(),
            product_name: "Local Storm Report".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n".to_string(),
        };

        let result = parse(&mut product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1525219680000000,"event_type":"NwsLsr","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KTOP","point":{"lat":26.8,"lon":-80.64},"poly":null},"md":null,"outlook":null,"report":{"reporter":"TRAINED SPOTTER","hazard":"Tornado","magnitude":null,"units":null,"was_measured":null,"report_ts":1525219200000000},"text":"TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. RECLASSIFIED AS A TORNADO.","title":"KTOP reports Tornado","valid_ts":null,"warning":null,"watch":null}"#;
        assert!(serialized_result == expected);
    }

    #[test]
    fn parse_old_report_should_be_ok_none() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-03T00:08:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "LSR".to_string(),
            product_name: "Local Storm Report".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n".to_string(),
        };

        let result = parse(&mut product).unwrap();
        assert!(result.is_none());
    }

    // TODO get wind LSR, check existing wind vs wind dmg
    #[test]
    fn parse_wind_speed_report() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T00:08:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "LSR".to_string(),
            product_name: "Local Storm Report".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n".to_string(),
        };

        let result = parse(&mut product).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_wind_damage_report() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T00:08:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "LSR".to_string(),
            product_name: "Local Storm Report".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: "\n158 \nNWUS52 KMFL 311935\nLSRMFL\n\nPRELIMINARY LOCAL STORM REPORT...CORRECTED\nNATIONAL WEATHER SERVICE MIAMI FL\n701 PM CDT TUE MAY 1 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0700 PM     TORNADO          2 SE PAHOKEE            26.80N  80.64W\n05/01/2018                   PALM BEACH         FL   TRAINED SPOTTER \n\n            TRAINED SKYWARN SPOTTER OBSERVED FROM PAHOKEE A FUNNEL \n            CLOUD APPROXIMATELY 3 MILES SOUTHEAST OF PAHOKEE, \n            PARTIALLY RAIN-WRAPPED AND NEARLY STATIONARY. THE FUNNEL \n            EXTENDED TO NEARLY HALFWAY TO THE GROUND BEFORE LIFTING. \n            LOCATION RADAR-ESTIMATED/ADJUSTED. VIDEO RECEIVED OF \n            FUNNEL REACHING THE GROUND WITH DUST BEING KICKED UP. \n            RECLASSIFIED AS A TORNADO. \n\n\n&&\n\nCORRECTED EVENT...FATALITIES...INJURIES...REMARKS\n\nEVENT NUMBER MFL1800020\n\n$$\n\nSI\n\n\n\n".to_string(),
        };

        let result = parse(&mut product).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn parse_hail_report() {
        let mut product = Product {
            _id: "_id".to_string(),
            id: "id".to_string(),
            wmo_collective_id: "NWUS54".to_string(),
            issuing_office: "KSJT".to_string(),
            issuance_time: "2018-03-27T21:18:00+00:00".to_string(),
            product_code: "LSR".to_string(),
            product_name: "Local Storm Report".to_string(),
            product_text: "\n106 \nNWUS54 KSJT 270116\nLSRSJT\n\nPRELIMINARY LOCAL STORM REPORT\nNational Weather Service San Angelo Tx\n316 PM CST MON MAR 27 2018\n\n..TIME...   ...EVENT...      ...CITY LOCATION...     ...LAT.LON...\n..DATE...   ....MAG....      ..COUNTY LOCATION..ST.. ...SOURCE....\n            ..REMARKS..\n\n0316 PM     HAIL             1 E SILVER              32.07N 100.66W\n03/27/2018  E1.25 INCH       COKE               TX   STORM CHASER    \n\n            1.25 HAIL ON HWY 208 NEAR SILVER \n\n\n&&\n\nEVENT NUMBER SJT1800032\n\n$$\n\nSJT\n\n".to_string(),
        };

        let result = parse(&mut product).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = r#"{"event_ts":1522185480000000,"event_type":"NwsLsr","expires_ts":null,"fetch_status":null,"image_uri":null,"ingest_ts":0,"location":{"wfo":"KSJT","point":{"lat":32.07,"lon":-100.66},"poly":null},"md":null,"outlook":null,"report":{"reporter":"STORM CHASER","hazard":"Hail","magnitude":1.25,"units":"Inches","was_measured":false,"report_ts":1522185360000000},"text":"1.25 HAIL ON HWY 208 NEAR SILVER","title":"KSJT reports 1.25 INCH Hail","valid_ts":null,"warning":null,"watch":null}"#;
        assert_eq!(serialized_result, expected);
    }
}
