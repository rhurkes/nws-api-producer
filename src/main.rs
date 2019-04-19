#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;

mod afd_parser;
mod domain;
mod lsr_parser;
mod parser;
mod tor_parser;
mod util;

use self::domain::{ListProduct, Product, ProductsResult};
use self::util::Config;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use wx::domain::Event;
use wx::util::Logger;

// TODO hook this back up
// TODO get list of products to parse
// TODO thread for each request

const APP_NAME: &str = "nws_api_loader";

fn main() {
    let config = Config::new("config.toml");
    let logger = Logger::new(&APP_NAME);

    info!(logger, "initializing"; "config" => serde_json::to_string(&config).unwrap());

    let age_limit_ms = config.age_limit_min * 60 * 1000;
    let mut threads = vec![];
    let config = Arc::new(config);
    let logger = Arc::new(logger);
    let product_codes = vec!["tor", "afd", "lsr"];

    for product_code in product_codes {
        let config = config.clone();
        let logger = logger.clone();

        threads.push(thread::spawn(move || {
            let client = reqwest::Client::new();
            let fetcher = util::Fetcher::new(&client, &config, &logger);
            let mut last_product_ts = wx::util::get_system_millis() - age_limit_ms;

            loop {
                let url = format!("{}/products/types/{}", config.api_host, product_code);

                if let Ok(product_list) = fetcher.fetch::<ProductsResult>(&url) {
                    let products = get_new_products(last_product_ts, product_list);

                    if !products.is_empty() {
                        last_product_ts =
                            wx::util::ts_to_ticks(&products[0].issuance_time).unwrap();
                    }

                    // let events: Vec<Option<Event>> = products
                    //     .iter()
                    //     // TODO these filter_maps are swallowing errors
                    //     .filter_map(|x| fetcher.fetch::<Product>(&x._id).ok())
                    //     .filter_map(|product| {
                    //         dbg!(&product);
                    //         parser.parse(&product).ok()
                    //     });
                    // TODO handle Result<Option<>>
                    // .collect();

                    // dbg!(events);
                }

                thread::sleep(Duration::from_millis(config.poll_interval_ms));
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
