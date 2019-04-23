use super::domain::Product;
use std::fs::File;
use std::io::Read;

#[allow(dead_code)]
pub fn get_product_from_file(path: &str) -> Product {
    let mut f = File::open(path).expect("product file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");
    let product: Product = serde_json::from_str(&contents).unwrap();
    product
}
