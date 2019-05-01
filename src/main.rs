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
use wx::util::Logger;

const APP_NAME: &str = "nws_api_loader";
const API_HOST: &str = "https://api.weather.gov";
const POLL_INTERVAL_MS: u64 = 60_000;
const USER_AGENT: &str = "sigtor.org";

fn main() {
    let logger = Logger::new(&APP_NAME);
    let mut threads = vec![];
    let logger = Arc::new(logger);
    let product_codes = vec!["afd", "ffw", "lsr", "sel", "svr", "svs", "swo", "tor"];
    info!(logger, "initializing"; "poll_interval_ms" => POLL_INTERVAL_MS);

    for product_code in product_codes {
        let logger = logger.clone();

        threads.push(thread::spawn(move || {
            let client = reqwest::Client::new();
            let store_client = wx::store::Client::new();
            let fetcher = util::Fetcher::new(&client, &logger, USER_AGENT);
            let mut last_product_ts = wx::util::get_system_micros();

            loop {
                let url = format!("{}/products/types/{}", API_HOST, product_code);

                if let Ok(product_list) = fetcher.fetch::<ProductsResult>(&url) {
                    let products = get_new_products(last_product_ts, product_list);

                    if !products.is_empty() {
                        let new_ts = wx::util::ts_to_ticks(&products[0].issuance_time).unwrap();
                        last_product_ts = new_ts;
                    }

                    products
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
                        .for_each(|x| match store_client.put_event(&x) {
                            Ok(_) => debug!(logger, "Stored event";),
                            Err(_) => {
                                error!(logger, "Store error"; "error" => "unable to store event")
                            }
                        });
                }

                thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
        }));
    }

    for thread in threads {
        let _ = thread.join();
    }
}

/**
 * Returns products newer than the latest seen. A simple take_while could suffice, but that
 * carries the possibility of missing products due to an unparseable datetime string.
 */
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
    use std::fs::File;
    use std::io::Read;

    #[test]
    fn get_new_products_should_only_parse_newer_products() {
        let last_ts = 1555977060000000;
        let mut f = File::open("data/product-list-tor").expect("product file not found");
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");
        let result: ProductsResult = serde_json::from_str(&contents).unwrap();
        let new_products = get_new_products(last_ts, result);
        let ids: Vec<&str> = new_products.iter().map(|x| x.id.as_str()).collect();
        let expected_ids = vec![
            "e0fdf7de-6229-4330-9d4d-3a2af96ffa4c",
            "e0fdf7de-6229-4330-9d4d-3a2af96ffa4d",
        ];
        assert_eq!(expected_ids, ids);
    }
}
