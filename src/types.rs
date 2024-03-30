use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Match {
    pub transaction_index: u64,
    pub transaction_id: String,
    pub output_index: u64,
    pub address: String,
    pub value: MatchValue,
    #[serde(flatten)]
    pub datum: Option<MatchDatum>,
    pub script_hash: Option<String>,
    pub created_at: Option<BlockReference>,
    pub spent_at: Option<BlockReference>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MatchDatum {
    #[serde(rename = "type")]
    pub typ: String,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MatchValue {
    pub coins: u64,
    pub assets: BTreeMap<String, u64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlockReference {
    pub slot_no: u64,
    pub header_hash: String,
}
