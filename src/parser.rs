use super::domain::{Product, Coordinates, WxEventMessage};
use super::products::{
    TornadoProduct,
};
use super::util;
use std::io::{ErrorKind, Read};
use regex::{Match, Regex};

pub struct RegexParser {
    pub description_regex: Regex,
    pub movement_regex: Regex,
    pub polygon_regex: Regex,
    pub source_regex: Regex,
    pub warning_for_regex: Regex,
}

impl RegexParser {
    pub fn new() -> RegexParser {
        let description_pattern = r"\n\*\s(at\s[\S|\s]+?)\n\n";
        let movement_pattern = r"\ntime...mot...loc\s(?P<time>\d{4}z)\s(?P<deg>\d+)\D{3}\s(?P<kt>\d+)kt\s(?P<lat>\d{4})\s(?P<lon>\d{4})";
        let polygon_pattern = r"lat...lon\s(?P<poly>\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s*?)";
        let source_pattern = r"\n{2}\s{2}source...(?P<src>.+)\.\s?\n{2}";
        let warning_for_pattern = r"\n\n\*\s[\s|\S]+ warning for\.{3}\n([\s|\S]*?)\n\n\*";

        RegexParser{
            description_regex: Regex::new(description_pattern).unwrap(),
            movement_regex: Regex::new(movement_pattern).unwrap(),
            polygon_regex: Regex::new(polygon_pattern).unwrap(),
            source_regex: Regex::new(source_pattern).unwrap(),
            warning_for_regex: Regex::new(warning_for_pattern).unwrap(),
        }
    }

    pub fn parse(&self, product: &Product) -> Result<WxEventMessage, Box<std::error::Error>> {
        // all parsing expects product text to be normalized to lower case
        let mut product_text = product.product_text.unwrap();
        product_text.make_ascii_lowercase();

        // all parsers have minimal error handling, relying on tight regexes to ensure valid input
        let result: Result<String, Box<std::error::Error>> = match product.product_code.as_ref() {
            "tor" => self.parse_tornado_product(&product_text),
            _ => Err(Box::new(std::io::Error::new(ErrorKind::Other, "unknown product code")))
        };

        Ok(WxEventMessage{
            data: result?,
            data_type: &format!("nws-api-{}", product.product_code),
            event_ts: util::ts_to_ticks(&product.issuance_time)?,
            ingest_ts: util::get_system_millis(),
            key: "TODO".to_string(),
            src: "nws-api",
        })
    }

    pub fn parse_tornado_product(&self, product: &str) -> Result<String, Box<std::error::Error>> {
        let movement = self.movement_regex.captures(&product);
        let description = self.description_regex.captures(&product);
        let polygon = self.polygon_regex.captures(&product);
        let source = self.source_regex.captures(&product);
        let description = description.unwrap();
        let movement = movement.unwrap();
        let polygon = polygon.unwrap();
        let source = source.unwrap();
        let lat = str_to_latlon(cap(movement.name("lat")), false);
        let lon = str_to_latlon(cap(movement.name("lon")), true);
        let polygon = cap(polygon.name("poly"));

        let polygon = vec![
            Coordinates{lat: str_to_latlon(&polygon[0..4], false), lon: str_to_latlon(&polygon[5..9], true)},
            Coordinates{lat: str_to_latlon(&polygon[10..14], false), lon: str_to_latlon(&polygon[15..19], true)},
            Coordinates{lat: str_to_latlon(&polygon[20..24], false), lon: str_to_latlon(&polygon[25..29], true)},
            Coordinates{lat: str_to_latlon(&polygon[30..34], false), lon: str_to_latlon(&polygon[35..39], true)},
        ];

        let product = TornadoProduct{
            source: cap(source.name("src")).to_string(),
            time: cap(movement.name("time")).to_string(),
            location: Coordinates{lat, lon},
            description: description[1].to_string(),
            motion_deg: cap(movement.name("deg")).parse::<u16>().unwrap(),
            motion_kt: cap(movement.name("kt")).parse::<u16>().unwrap(),
            polygon,
            is_pds: product.contains("particularly dangerous situation"),
            is_observed: product.contains("tornado...observed"),
            is_tornado_emergency: product.contains("tornado emergency"),
        };

        Ok(serde_json::to_string(&product)?)
    }
}

pub fn cap(m: Option<Match>) -> &str {
    m.unwrap().as_str()
}

pub fn str_to_latlon(input: &str, invert: bool) -> f32 {
    let sign = if invert { -1.0 } else { 1.0 };
    input.parse::<f32>().unwrap() / 100.0 * sign
}

// TODO build helper for creating test products
pub fn get_test_product() {

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tornado_product() {
        let parser = RegexParser::new();
        let product = Product{
            _id: "_id".to_string(),
            id: "id".to_string(),
            issuance_time: "2018-05-02T01:01:00+00:00".to_string(),
            issuing_office: "KTOP".to_string(),
            product_code: "TOR".to_string(),
            product_name: "Tornado Warning".to_string(),
            wmo_collective_id: "WFUS53".to_string(),
            product_text: Some(r"\n271 \nWFUS53 KTOP 020101\nTORTOP\nKSC027-161-201-020145-\n/O.NEW.KTOP.TO.W.0009.180502T0101Z-180502T0145Z/\n\nBULLETIN - EAS ACTIVATION REQUESTED\nTornado Warning\nNational Weather Service Topeka KS\n801 PM CDT TUE MAY 1 2018\n\nThe National Weather Service in Topeka has issued a\n\n* Tornado Warning for...\n  Northwestern Riley County in northeastern Kansas...\n  Southern Washington County in north central Kansas...\n  Northern Clay County in north central Kansas...\n\n* Until 845 PM CDT\n    \n* At 800 PM CDT, a large and extremely dangerous tornado was located\n  2 miles south of Clifton, moving northeast at 25 mph.\n\n  This is a PARTICULARLY DANGEROUS SITUATION. TAKE COVER NOW! \n\n  HAZARD...Damaging tornado. \n\n  SOURCE...Radar indicated rotation. \n\n  IMPACT...You are in a life-threatening situation. Flying debris \n           may be deadly to those caught without shelter. Mobile \n           homes will be destroyed. Considerable damage to homes, \n           businesses, and vehicles is likely and complete \n           destruction is possible. \n\n* The tornado will be near...\n  Morganville around 805 PM CDT. \n  Palmer around 820 PM CDT. \n  Linn around 830 PM CDT. \n  Greenleaf around 845 PM CDT. \n\nPRECAUTIONARY/PREPAREDNESS ACTIONS...\n\nTo repeat, a large, extremely dangerous and potentially deadly\ntornado is developing. To protect your life, TAKE COVER NOW! Move to\na basement or an interior room on the lowest floor of a sturdy\nbuilding. Avoid windows. If you are outdoors, in a mobile home, or in\na vehicle, move to the closest substantial shelter and protect\nyourself from flying debris.\n\nTornadoes are extremely difficult to see and confirm at night. Do not\nwait to see or hear the tornado. TAKE COVER NOW!\n\n&&\n\nLAT...LON 3977 9697 3950 9680 3939 9737 3959 9737\nTIME...MOT...LOC 0100Z 245DEG 24KT 3952 9728 \n\nTORNADO...RADAR INDICATED\nTORNADO DAMAGE THREAT...CONSIDERABLE\nHAIL...2.00IN\n\n$$\n\nBaerg\n\n".to_string()),
        };
        let result = parser.parse_tornado_product(&product.product_text.unwrap()).unwrap();
        let serialized_result = serde_json::to_string(&result).unwrap();
        let expected = "{\"is_pds\":true,\"is_observed\":false,\"is_tornado_emergency\":false,\"source\":\"radar indicated rotation\",\"description\":\"at 800 pm cdt, a large and extremely dangerous tornado was located\\n  2 miles south of clifton, moving northeast at 25 mph.\",\"polygon\":[{\"lat\":39.77,\"lon\":-96.97},{\"lat\":39.5,\"lon\":-96.8},{\"lat\":39.39,\"lon\":-97.37},{\"lat\":39.59,\"lon\":-97.37}],\"location\":{\"lat\":39.52,\"lon\":-97.28},\"time\":\"0100z\",\"motion_deg\":245,\"motion_kt\":24}";
        
        assert!(serialized_result == expected);
    }
}
