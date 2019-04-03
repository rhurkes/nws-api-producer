use super::afd_parser;
use super::domain::Product;
use super::lsr_parser;
use super::tor_parser;
use chrono::prelude::*;
use regex::{Match, Regex, RegexBuilder};
use wx::domain::{Coordinates, Event, EventType, HazardType, Location, Report, Warning, Watch};
use wx::error::{Error, WxError};
use wx::util;

pub struct RegexParser {
    description_regex: Regex,
    movement_regex: Regex,
    polygon_regex: Regex,
    source_regex: Regex,
    issued_for_regex: Regex,
    valid_regex: Regex,
}

impl RegexParser {
    pub fn new() -> RegexParser {
        let description_pattern = r"\n\*\s(?P<desc>at\s[\S|\s]+?)\n\n";
        let movement_pattern = r"\ntime...mot...loc\s(?P<time>\d{4}z)\s(?P<deg>\d+)\D{3}\s(?P<kt>\d+)kt\s(?P<lat>\d{4})\s(?P<lon>\d{4})";
        let polygon_pattern =
            r"lat...lon\s(?P<poly>\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\s*?)";
        let source_pattern = r"\n{2}\s{2}source...(?P<src>.+)\.\s?\n{2}";
        let issued_for_pattern = r"\n\n\*\s[\s|\S]+ warning for\.{3}\n(?P<for>[\s|\S]*?)\n\n\*";
        let valid_pattern = r"(\d{6}t\d{4}z)-(\d{6}t\d{4}z)";

        RegexParser {
            description_regex: RegexBuilder::new(description_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            movement_regex: RegexBuilder::new(movement_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            polygon_regex: RegexBuilder::new(polygon_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            source_regex: RegexBuilder::new(source_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            issued_for_regex: RegexBuilder::new(issued_for_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
            valid_regex: RegexBuilder::new(valid_pattern)
                .case_insensitive(true)
                .build()
                .unwrap(),
        }
    }

    pub fn parse(&self, product: &Product) -> Result<Option<Event>, Error> {
        match product.product_code.as_ref() {
            "AFD" => afd_parser::parse(&product),
            "LSR" => lsr_parser::parse(&product),
            "TOR" => tor_parser::parse(&product),
            _ => {
                let reason = format!("unknown product code: {}", &product.product_code);
                Err(Error::Wx(<WxError>::new(&reason)))
            }
        }
    }
}

pub fn short_time_to_ticks(input: &str) -> Result<u64, Error> {
    Ok(Utc.datetime_from_str(input, "%y%m%dT%H%MZ")?.timestamp() as u64 * 1000)
}

pub fn get_parse_error(text: &str) -> Error {
    let reason = format!("unable to parse product: {}", text);
    Error::Wx(<WxError>::new(&reason))
}

pub fn get_src(product_code: &str) -> String {
    format!("nws-api-{}", product_code).to_string()
}

pub fn cap(m: Option<Match>) -> &str {
    m.unwrap().as_str()
}

pub fn str_to_latlon(input: &str, invert: bool) -> f32 {
    let sign = if invert { -1.0 } else { 1.0 };
    input.parse::<f32>().unwrap() / 100.0 * sign
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_time_to_ticks_should_return_correct_ticks() {
        let short_time = "190522T2100Z";
        let result = short_time_to_ticks(short_time).unwrap();
        assert_eq!(result, 1558558800000);
    }
}
