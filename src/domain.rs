#[derive(Debug, Deserialize)]
pub struct ProductsResult {
    #[serde(rename = "@context")]
    pub _context: Context,
    #[serde(rename = "@graph")]
    pub products: Vec<ListProduct>,
}

#[derive(Debug, Deserialize)]
pub struct Context {
    #[serde(rename = "@vocab")]
    pub _vocab: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListProduct {
    #[serde(rename = "@id")]
    pub _id: String,
    pub id: String,
    #[serde(rename = "wmoCollectiveId")]
    wmo_collective_id: String,
    #[serde(rename = "issuingOffice")]
    issuing_office: String,
    #[serde(rename = "issuanceTime")]
    pub issuance_time: String,
    #[serde(rename = "productCode")]
    pub product_code: String,
    #[serde(rename = "productName")]
    product_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    #[serde(rename = "@id")]
    pub _id: String,
    pub id: String,
    #[serde(rename = "wmoCollectiveId")]
    pub wmo_collective_id: String,
    #[serde(rename = "issuingOffice")]
    pub issuing_office: String,
    #[serde(rename = "issuanceTime")]
    pub issuance_time: String,
    #[serde(rename = "productCode")]
    pub product_code: String,
    #[serde(rename = "productName")]
    pub product_name: String,
    #[serde(rename = "productText")]
    pub product_text: String,
}
