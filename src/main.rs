#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;

mod afd_parser;
mod domain;
mod ffw_parser;
mod lsr_parser;
mod parser;
mod sel_parser;
mod svr_parser;
mod svs_parser;
mod swo_parser;
mod test_util;
mod tor_parser;
mod util;

use self::domain::{ListProduct, Product, ProductsResult};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wx::domain::Event;
use wx::util::Logger;

const APP_NAME: &str = "nws_api_loader";
const API_HOST: &str = "https://api.weather.gov";
const POLL_INTERVAL_MS: u64 = 60_000;
const USER_AGENT: &str = "sigtor.org";

fn main() {
    let logger = Logger::new(&APP_NAME);
    let mut threads = vec![];
    let logger = Arc::new(logger);
    // let product_codes = vec!["afd", "ffw", "lsr", "sel", "svr", "svs", "swo", "tor"];
    let product_codes = vec!["afd"];
    info!(logger, "initializing"; "poll_interval_ms" => POLL_INTERVAL_MS);

    for product_code in product_codes {
        let logger = logger.clone();

        threads.push(thread::spawn(move || {
            let client = reqwest::Client::new();
            let fetcher = util::Fetcher::new(&client, &logger, USER_AGENT);
            // let mut last_product_ts = wx::util::get_system_micros();
            let mut last_product_ts = 0;

            loop {
                let url = format!("{}/products/types/{}", API_HOST, product_code);

                if let Ok(product_list) = fetcher.fetch::<ProductsResult>(&url) {
                    let products = get_new_products(last_product_ts, product_list);

                    if !products.is_empty() {
                        let new_ts = wx::util::ts_to_ticks(&products[0].issuance_time).unwrap();
                        last_product_ts = new_ts;
                    }

                    let events: Vec<Event> = products
                        .iter()
                        .map(|x| match fetcher.fetch::<Product>(&x._id) {
                            Ok(value) => Some(value),
                            Err(error) => {
                                error!(logger, "Fetch error"; "error" => format!("{}", error));
                                None
                            }
                        })
                        .filter(Option::is_some)
                        .map(|x| match parser::parse(&x.unwrap()) {
                            Ok(value) => Some(value),
                            Err(error) => {
                                error!(logger, "Parsing error"; "error" => format!("{}", error));
                                None
                            }
                        })
                        .filter_map(Option::unwrap)
                        .collect();

                    if !events.is_empty() {
                        println!("writing {} events", events.len());
                    }
                    // TODO write to RocksDB
                }

                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
        }));
    }

    for thread in threads {
        let _ = thread.join();
    }
}

fn get_new_products(last_ts: u64, products_result: ProductsResult) -> Vec<ListProduct> {
    let mut new_products: Vec<ListProduct> = vec![];

    for product in products_result.products {
        if let Ok(ticks) = wx::util::ts_to_ticks(&product.issuance_time) {
            if ticks <= last_ts {
                break;
            }
            new_products.push(product);
        }
    }

    new_products
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn get_new_product_ids_should_handle_no_product_ids() {
    //     let product_list = ProductsResult {
    //         _context: Context {
    //             _vocab: String::new(),
    //         },
    //         products: vec![],
    //     };
    //     let result = get_new_product_ids(0, product_list);
    //     let expected: Vec<Product> = vec![];
    //     assert!(result.len() == expected.len());
    // }

    // #[test]
    // fn get_new_product_ids_should_filter_out_same_or_older_products() {
    //     let product_list_result = ProductsResult{_context: Context{_vocab: String::new()}, products: vec![
    //         serde_json::from_str(r#"{"@id": "", "id": "", "issuanceTime": "2018-12-01T00:23:59+00:00", "issuingOffice": "", "productCode": "", "productName": "", "wmoCollectiveId": ""}"#).unwrap(),
    //         serde_json::from_str(r#"{"@id": "", "id": "", "issuanceTime": "2018-12-01T00:23:00+00:00", "issuingOffice": "", "productCode": "", "productName": "", "wmoCollectiveId": ""}"#).unwrap(),
    //         serde_json::from_str(r#"{"@id": "", "id": "", "issuanceTime": "2018-12-01T00:22:59+00:00", "issuingOffice": "", "productCode": "", "productName": "", "wmoCollectiveId": ""}"#).unwrap(),
    //     ]};

    //     let result = get_new_product_ids(1543623780000, product_list_result);
    //     let mut result_times: Vec<String> = vec![];
    //     for product in result {
    //         result_times.push(product.issuance_time);
    //     }

    //     assert!(result_times == ["2018-12-01T00:23:59+00:00"]);
    // }
}
