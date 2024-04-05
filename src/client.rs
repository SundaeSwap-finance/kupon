use serde::Deserialize;
use url::Url;

use crate::{errors::KuponError, types::Match};

const DEFAULT_ENDPOINT: &str = "http://localhost:1442";

#[derive(Default)]
pub struct Builder {
    endpoint: Option<String>,
}

impl Builder {
    pub fn with_endpoint<T: Into<String>>(endpoint: T) -> Self {
        Self {
            endpoint: Some(endpoint.into()),
        }
    }

    pub fn build(self) -> Result<Client, KuponError> {
        let endpoint = self.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
        let endpoint = Url::parse(endpoint)?;
        let client = reqwest::ClientBuilder::new().build()?;
        Ok(Client { client, endpoint })
    }
}

pub struct Client {
    client: reqwest::Client,
    endpoint: Url,
}

impl Client {
    pub async fn matches(&self, options: &MatchOptions<'_>) -> Result<Vec<Match>, KuponError> {
        let match_url = options.to_url(&self.endpoint)?;
        let request = self.client.get(match_url).build()?;
        let response = self.client.execute(request).await?.json().await?;
        match response {
            MatchResponse::Success(matches) => Ok(matches),
            MatchResponse::Failure { hint } => Err(KuponError::KupoError(hint)),
        }
    }

    pub async fn datum(&self, hash: &str) -> Result<Option<String>, KuponError> {
        let mut datum_url = self.endpoint.clone();
        datum_url.set_path(&format!("v1/datums/{}", hash));
        let request = self.client.get(datum_url).build()?;
        let response = self.client.execute(request).await?.json().await?;
        match response {
            Some(DatumResponse::Success { datum }) => Ok(Some(datum)),
            Some(DatumResponse::Failure { hint }) => Err(KuponError::KupoError(hint)),
            None => Ok(None),
        }
    }
}

#[derive(Clone, Debug)]
enum SpentStatus {
    Unspent,
    Spent,
}

#[derive(Clone, Debug)]
struct AssetIdOptions<'a> {
    policy_id: &'a str,
    asset_name: Option<&'a str>,
}

impl<'a> AssetIdOptions<'a> {
    pub(crate) fn to_pattern(&self) -> String {
        match self.asset_name {
            Some(name) => format!("{}.{}", self.policy_id, name),
            None => format!("{}.*", self.policy_id),
        }
    }
}

#[derive(Clone, Debug)]
struct TransactionIdOptions<'a> {
    transaction_id: &'a str,
    output_index: Option<u64>,
}

impl<'a> TransactionIdOptions<'a> {
    pub(crate) fn to_pattern(&self) -> String {
        match self.output_index {
            Some(index) => format!("{}@{}", index, self.transaction_id),
            None => format!("*@{}", self.transaction_id),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MatchOptions<'a> {
    spent_status: Option<SpentStatus>,
    address: Option<&'a str>,
    asset: Option<AssetIdOptions<'a>>,
    transaction: Option<TransactionIdOptions<'a>>,
}

impl<'a> MatchOptions<'a> {
    pub fn only_spent(self) -> Self {
        Self {
            spent_status: Some(SpentStatus::Spent),
            ..self
        }
    }

    pub fn only_unspent(self) -> Self {
        Self {
            spent_status: Some(SpentStatus::Unspent),
            ..self
        }
    }

    pub fn address(self, address: &'a str) -> Self {
        Self {
            address: Some(address),
            ..self
        }
    }

    pub fn policy_id(self, policy_id: &'a str) -> Self {
        Self {
            asset: Some(AssetIdOptions {
                policy_id,
                asset_name: None,
            }),
            ..self
        }
    }

    pub fn asset_id(self, asset_id: &'a str) -> Self {
        let (policy_id, asset_name) = match asset_id.split_once('.') {
            Some((policy_id, asset_name)) => (policy_id, Some(asset_name)),
            None => (asset_id, None),
        };
        Self {
            asset: Some(AssetIdOptions {
                policy_id,
                asset_name,
            }),
            ..self
        }
    }

    pub fn transaction(self, transaction_id: &'a str) -> Self {
        Self {
            transaction: Some(TransactionIdOptions {
                transaction_id,
                output_index: None,
            }),
            ..self
        }
    }

    pub fn transaction_output(self, transaction_id: &'a str, index: u64) -> Self {
        Self {
            transaction: Some(TransactionIdOptions {
                transaction_id,
                output_index: Some(index),
            }),
            ..self
        }
    }

    pub(crate) fn to_url(&self, endpoint: &Url) -> Result<Url, KuponError> {
        let mut url = endpoint.clone();

        let mut query = url.query_pairs_mut();

        let mut pattern = self.address.map(|s| s.to_owned());

        if let Some(transaction) = &self.transaction {
            if pattern.is_none() {
                pattern = Some(transaction.to_pattern());
            } else {
                query.append_pair("transaction_id", transaction.transaction_id);
                if let Some(index) = transaction.output_index {
                    query.append_pair("output_index", &index.to_string());
                }
            }
        }

        if let Some(asset) = &self.asset {
            if pattern.is_none() {
                pattern = Some(asset.to_pattern());
            } else {
                query.append_pair("policy_id", asset.policy_id);
                if let Some(asset_name) = asset.asset_name {
                    query.append_pair("asset_name", asset_name);
                }
            }
        }

        match self.spent_status {
            Some(SpentStatus::Spent) => {
                query.append_key_only("spent");
            }
            Some(SpentStatus::Unspent) => {
                query.append_key_only("unspent");
            }
            None => {}
        };

        drop(query);

        if let Some(pattern) = pattern {
            url.set_path(&format!("matches/{}", pattern));
        } else {
            url.set_path("matches");
        }

        Ok(url)
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MatchResponse {
    Success(Vec<Match>),
    Failure { hint: String },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum DatumResponse {
    Success { datum: String },
    Failure { hint: String },
}
