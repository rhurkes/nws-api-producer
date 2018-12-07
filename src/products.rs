use super::domain::{Coordinates};

#[derive(Debug, Deserialize, Serialize)]
pub struct TornadoProduct {
    pub is_pds: bool,
    pub is_observed: bool,
    pub is_tornado_emergency: bool,
    pub source: String,
    pub description: String,
    pub polygon: Vec<Coordinates>,
    pub location: Coordinates,
    pub time: String,
    pub motion_deg: u16,
    pub motion_kt: u16,
}
