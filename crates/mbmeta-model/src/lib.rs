use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Artist {
    pub id: uuid::Uuid,
    pub oldids: Vec<String>,
    pub artistname: String,
    pub sortname: String,
    pub artistaliases: Vec<String>,
    pub status: String,
    pub disambiguation: String,
    #[serde(rename = "type")]
    pub type_field: Option<String>,
    pub rating: Rating,
    pub links: Vec<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rating {
    #[serde(rename = "Count")]
    pub count: Option<u32>,
    #[serde(rename = "Value")]
    pub value: Option<f64>,
}
