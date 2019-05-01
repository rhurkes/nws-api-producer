use super::afd_parser;
use super::domain::Product;
use super::ffw_parser;
use super::lsr_parser;
use super::sel_parser;
use super::svr_parser;
use super::svs_parser;
use super::swo_parser;
use super::tor_parser;
use chrono::prelude::*;
use regex::{Match, Regex, RegexBuilder};
use std::panic;
use wx::domain::Event;
use wx::error::{Error, WxError};

pub struct Regexes {
    pub movement: Regex,
    pub poly_condensed: Regex,
    pub source: Regex,
    pub valid: Regex,
    pub affected: Regex,
    pub probability: Regex,
    pub wfos: Regex,
    pub md_number: Regex,
    pub watch_id: Regex,
    pub poly: Regex,
    pub warning_for: Regex,
    pub watch_for: Regex,
}

impl Regexes {
    pub fn new() -> Regexes {
        let movement_pattern = r"\ntime...mot...loc\s(?P<time>\d{4}z)\s(?P<deg>\d+)\D{3}\s(?P<kt>\d+)kt\s(?P<lat>\d{4})\s(?P<lon>\d{4,5})";
        let source_pattern = r"\n{2}\s{2}source...(?P<src>.+)\.\s?\n{2}";
        let valid_pattern = r"(\d{6}t\d{4}z)-(\d{6}t\d{4}z)";
        let affected_pattern = r"Areas affected\.{3}([\S|\s]*?)\n\n";
        let probability_pattern = r"Probability of Watch Issuance...(\d{1,3}) percent";
        let wfos_pattern = r"ATTN...WFO...(.+)\n\n";
        let poly_pattern = r"(\d{4}\s\d{4,5})+";
        let poly_condensed_pattern = r"(\d{8})\s";
        let md_number_pattern = r"Mesoscale Discussion (\d{4})";
        let watch_id_pattern = r"Watch Number (\d{1,3})";
        let warning_for_pattern = r"Warning for...([\s|\S]+?)\n\n";
        let watch_for_pattern = r"Watch for portions of\s\n([\s|\S]+?)\n\n";

        Regexes {
            movement: RegexBuilder::new(movement_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            poly_condensed: RegexBuilder::new(poly_condensed_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            source: RegexBuilder::new(source_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            valid: RegexBuilder::new(valid_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            affected: RegexBuilder::new(affected_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            probability: RegexBuilder::new(probability_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            wfos: RegexBuilder::new(wfos_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            poly: RegexBuilder::new(poly_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            md_number: RegexBuilder::new(md_number_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            watch_id: RegexBuilder::new(watch_id_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            warning_for: RegexBuilder::new(warning_for_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            watch_for: RegexBuilder::new(watch_for_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
        }
    }
}

/**
 * Function that determines which product gets which parser.
 * NOTE: We're forcing no abort panics in the Cargo.toml, and catching panics here.
 * Yes, this is UGLY and BAD - but there are a ton of slices and regex Option unwraps
 * buried within. One rainy day, I'll start validating the correctness, but for now,
 * it's critical that the processing threads don't die.
 */
pub fn parse(product: &Product) -> Result<Option<Event>, Error> {
    let regexes = Regexes::new();

    let result = panic::catch_unwind(|| match product.product_code.as_ref() {
        "AFD" => afd_parser::parse(&product),
        "LSR" => lsr_parser::parse(&product),
        "SEL" => sel_parser::parse(&product, regexes),
        "SVR" => svr_parser::parse(&product, regexes),
        "SVS" => svs_parser::parse(&product),
        "SWO" => swo_parser::parse(&product, regexes),
        "TOR" => tor_parser::parse(&product, regexes),
        "FFW" => ffw_parser::parse(&product, regexes),
        _ => {
            let reason = format!("unknown product code: {}", &product.product_code);
            Err(Error::Wx(<WxError>::new(&reason)))
        }
    });

    if result.is_ok() {
        result.unwrap()
    } else {
        let reason = format!("recovered from panic on product: {}", product.id);
        Err(Error::Wx(<WxError>::new(&reason)))
    }
}

pub fn short_time_to_ticks(input: &str) -> Result<u64, Error> {
    Ok(Utc.datetime_from_str(input, "%y%m%dT%H%MZ")?.timestamp() as u64 * 1_000_000)
}

pub fn get_parse_error(text: &str) -> Error {
    let reason = format!("unable to parse product: {}", text);
    Error::Wx(<WxError>::new(&reason))
}

pub fn cap(m: Option<Match>) -> &str {
    m.unwrap().as_str()
}

pub fn str_to_latlon(input: &str, invert: bool) -> f32 {
    let sign = if invert { -1.0 } else { 1.0 };
    let mut value = input.parse::<f32>().unwrap();
    // longitudes are inverted, and values over 100 can drop the '1'
    if invert && value < 5000.0 {
        value += 10000.0;
    }
    value / 100.0 * sign
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn str_to_latlon_should_parse_correctly() {
        let tests = vec![
            // input, invert, expected
            ("3000", false, 30.0),
            ("3156", false, 31.56),
            ("9234", true, -92.34),
            ("9000", true, -90.0),
            ("0156", true, -101.56),
            ("10156", true, -101.56),
        ];

        tests.iter().for_each(|x| {
            let result = str_to_latlon(x.0, x.1);
            assert_eq!(x.2, result);
        });
    }

    #[test]
    fn short_time_to_ticks_should_return_correct_ticks() {
        let short_time = "190522T2100Z";
        let result = short_time_to_ticks(short_time).unwrap();
        assert_eq!(result, 1558558800000000);
    }
}
