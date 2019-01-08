#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;

mod domain;
mod util;
mod parser;
mod products;

use self::domain::{Context, ListProduct, Product, ProductsResult};
use self::products::{TornadoProduct};
use self::util::{Config, KafkaProducer, Logger};
use std::sync::Arc;
use std::thread;

fn main() {
    let config = Config::new("config.toml");
    let logger = Logger::new();

    info!(logger, "initializing"; "config" => serde_json::to_string(&config).unwrap());

    let age_limit_ms = config.age_limit_min * 60 * 1000;
    let mut threads = vec![];
    let config = Arc::new(config);
    let logger = Arc::new(logger);
    let tasks = vec![WorkDefinition { product: "tor" }];

    for task in tasks {
        let config = config.clone();
        let logger = logger.clone();

        threads.push(thread::spawn(move || {
            let client = reqwest::Client::new();
            let fetcher = util::Fetcher::new(&client, &config, &logger);
            let parser = parser::RegexParser::new();
            let mut last_product_ts = util::get_system_millis() - age_limit_ms;

            loop {
                let start = util::get_system_millis();
                let url = format!("{}/products/types/{}", config.api_host, task.product);

                if let Ok(product_list) = fetcher.fetch::<ProductsResult>(&url) {
                    let products = get_new_products(last_product_ts, product_list);

                    if !products.is_empty() {
                        last_product_ts = util::ts_to_ticks(&products[0].issuance_time).unwrap();
                    }

                    let _: Vec<_> = products.iter()
                        .filter_map(|x| fetcher.fetch::<Product>(&x.id).ok())
                        // .filter_map(|x| {
                            // parser.parse needs to take a product and a task.product, and return a WxEventMessage
                            // return parser.parse_tornado_product(x.product_text.unwrap()).ok()
                        // })
                        .collect();
                }

                let elapsed_ms = util::get_system_millis() - start;
                let delay_ms = if elapsed_ms >= config.poll_interval_ms {
                    0
                } else {
                    config.poll_interval_ms - elapsed_ms
                };

                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }));
    }

    for thread in threads {
        let _ = thread.join();
    }
}

pub struct WorkDefinition<'a> {
    pub product: &'a str,
    // pub work: fn(&str, &str, &Config, &Client) -> ()
}

fn get_new_products(last_ts: u64, products_result: ProductsResult) -> Vec<ListProduct> {
    let mut new_products: Vec<ListProduct> = vec![];

    for product in products_result.products {
        if let Ok(ticks) = util::ts_to_ticks(&product.issuance_time) {
            println!("last_ts: {}, this: {}", last_ts, ticks);
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
