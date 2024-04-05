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
    pub datum: Option<DatumHash>,
    pub script_hash: Option<String>,
    pub created_at: Option<BlockReference>,
    pub spent_at: Option<BlockReference>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DatumHash {
    #[serde(rename = "datum_type")]
    pub typ: String,
    #[serde(rename = "datum_hash")]
    pub hash: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MatchValue {
    pub coins: u64,
    pub assets: BTreeMap<AssetId, u64>,
}

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct AssetId {
    pub policy_id: String,
    pub asset_name: Option<String>,
}
impl AssetId {
    pub fn from_hex(hex: &str) -> AssetId {
        let (policy_id, asset_name) = match hex.split_once('.') {
            Some((policy_id, asset_name)) => (policy_id, Some(asset_name)),
            None => (hex, None),
        };
        Self {
            policy_id: policy_id.into(),
            asset_name: asset_name.map(|a| a.to_owned()),
        }
    }

    pub fn to_hex(&self) -> String {
        match &self.asset_name {
            Some(name) => format!("{}.{}", self.policy_id, name),
            None => self.policy_id.to_string(),
        }
    }
}

impl<'de> Deserialize<'de> for AssetId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw_value: &str = Deserialize::deserialize(deserializer)?;
        Ok(Self::from_hex(raw_value))
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BlockReference {
    pub slot_no: u64,
    pub header_hash: String,
}

#[cfg(test)]
mod tests {
    use crate::{AssetId, MatchValue};

    #[test]
    fn should_deserialize_asset_id() {
        let raw_asset_id = r#""1d7f33bd23d85e1a25d87d86fac4f199c3197a2f7afeb662a0f34e1e.776f726c646d6f62696c65746f6b656e""#;
        let asset_id: AssetId = serde_json::from_str(raw_asset_id).unwrap();
        assert_eq!(
            asset_id.policy_id,
            "1d7f33bd23d85e1a25d87d86fac4f199c3197a2f7afeb662a0f34e1e"
        );
        assert_eq!(
            asset_id.asset_name,
            Some("776f726c646d6f62696c65746f6b656e".into())
        );
    }

    #[test]
    fn should_deserialize_asset_id_without_name() {
        let raw_asset_id = r#""a04ce7a52545e5e33c2867e148898d9e667a69602285f6a1298f9d68""#;
        let asset_id: AssetId = serde_json::from_str(raw_asset_id).unwrap();
        assert_eq!(
            asset_id.policy_id,
            "a04ce7a52545e5e33c2867e148898d9e667a69602285f6a1298f9d68"
        );
        assert_eq!(asset_id.asset_name, None);
    }

    #[test]
    fn should_deserialize_match_value() {
        let match_value = r#"
            {
                "coins": 1000,
                "assets": {
                    "1d7f33bd23d85e1a25d87d86fac4f199c3197a2f7afeb662a0f34e1e.776f726c646d6f62696c65746f6b656e": 3
                }
            }
        "#;

        let expected_asset_id = AssetId {
            policy_id: "1d7f33bd23d85e1a25d87d86fac4f199c3197a2f7afeb662a0f34e1e".into(),
            asset_name: Some("776f726c646d6f62696c65746f6b656e".into()),
        };

        let value: MatchValue = serde_json::from_str(match_value).unwrap();
        assert_eq!(value.coins, 1000);
        assert_eq!(value.assets[&expected_asset_id], 3);
    }
}
