use std::time::Duration;

use rand::{thread_rng, Rng};
use serde::Deserialize;
use tokio::time::sleep;
use url::Url;

use crate::{errors::KuponError, Health, HealthStatus, Match, ServerInfo};

const DEFAULT_ENDPOINT: &str = "http://localhost:1442";

#[derive(Default)]
pub struct Builder {
    endpoint: Option<String>,
    retries: usize,
}

impl Builder {
    pub fn with_endpoint<T: Into<String>>(endpoint: T) -> Self {
        Self {
            endpoint: Some(endpoint.into()),
            retries: 0,
        }
    }

    pub fn with_retries(self, retries: usize) -> Self {
        Self { retries, ..self }
    }

    pub fn build(self) -> Result<Client, KuponError> {
        let endpoint = self.endpoint.as_deref().unwrap_or(DEFAULT_ENDPOINT);
        let endpoint = Url::parse(endpoint)?;
        let client = reqwest::ClientBuilder::new().build()?;
        Ok(Client {
            client,
            endpoint,
            retries: self.retries,
        })
    }
}

pub struct Client {
    client: reqwest::Client,
    endpoint: Url,
    retries: usize,
}

impl Client {
    pub async fn health(&self) -> Health {
        match self.try_health().await {
            Ok(health) => health,
            Err(error) => Health {
                status: HealthStatus::Error(error.to_string()),
                info: None,
            },
        }
    }

    async fn try_health(&self) -> Result<Health, KuponError> {
        let mut health_url = self.endpoint.clone();
        health_url.set_path("health");

        let request = self
            .client
            .get(health_url)
            .header("Accept", "application/json")
            .build()?;
        let response = self.client.execute(request).await?;
        let mut status = match response.status().as_u16() {
            200 => HealthStatus::Healthy,
            202 => HealthStatus::Syncing,
            503 => HealthStatus::Disconnected,
            other => HealthStatus::Error(format!("Unexpected response code {}", other)),
        };
        let info = match response.json().await {
            Ok(HealthResponse::Success(info)) => Some(info),
            Ok(HealthResponse::Failure { hint }) => {
                status = HealthStatus::Error(hint);
                None
            }
            Err(_) => None,
        };
        Ok(Health { status, info })
    }

    pub async fn matches(&self, options: &MatchOptions) -> Result<Vec<Match>, KuponError> {
        let mut retries = self.retries;
        let mut delay = Duration::from_millis(100);
        loop {
            let match_url = options.to_url(&self.endpoint)?;
            let request = self.client.get(match_url).build()?;
            let response = self.client.execute(request).await?;
            let status = response.status();
            match response.json().await? {
                MatchResponse::Success(matches) => return Ok(matches),
                MatchResponse::Failure { hint } => {
                    if retries == 0 || status.as_u16() != 503 {
                        return Err(KuponError::KupoError(hint));
                    }
                    sleep(delay).await;
                    retries -= 1;
                    delay = delay.mul_f32(thread_rng().gen_range(1.5..2.5))
                }
            };
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
struct AssetIdOptions {
    policy_id: String,
    asset_name: Option<String>,
}

impl AssetIdOptions {
    pub(crate) fn to_pattern(&self) -> String {
        match &self.asset_name {
            Some(name) => format!("{}.{}", self.policy_id, name),
            None => format!("{}.*", self.policy_id),
        }
    }
}

#[derive(Clone, Debug)]
struct TransactionIdOptions {
    transaction_id: String,
    output_index: Option<u64>,
}

impl TransactionIdOptions {
    pub(crate) fn to_pattern(&self) -> String {
        match self.output_index {
            Some(index) => format!("{}@{}", index, self.transaction_id),
            None => format!("*@{}", self.transaction_id),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MatchOptions {
    spent_status: Option<SpentStatus>,
    address: Option<String>,
    credential: Option<String>,
    asset: Option<AssetIdOptions>,
    transaction: Option<TransactionIdOptions>,
}

impl MatchOptions {
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

    pub fn address<T: Into<String>>(self, address: T) -> Self {
        Self {
            address: Some(address.into()),
            ..self
        }
    }

    pub fn credential<T: Into<String>>(self, credential: T) -> Self {
        Self {
            credential: Some(credential.into()),
            ..self
        }
    }

    pub fn policy_id<T: Into<String>>(self, policy_id: T) -> Self {
        Self {
            asset: Some(AssetIdOptions {
                policy_id: policy_id.into(),
                asset_name: None,
            }),
            ..self
        }
    }

    pub fn asset_id(self, asset_id: &str) -> Self {
        let (policy_id, asset_name) = match asset_id.split_once('.') {
            Some((policy_id, asset_name)) => (policy_id, Some(asset_name.into())),
            None => (asset_id, None),
        };
        Self {
            asset: Some(AssetIdOptions {
                policy_id: policy_id.into(),
                asset_name,
            }),
            ..self
        }
    }

    pub fn transaction<T: Into<String>>(self, transaction_id: T) -> Self {
        Self {
            transaction: Some(TransactionIdOptions {
                transaction_id: transaction_id.into(),
                output_index: None,
            }),
            ..self
        }
    }

    pub fn transaction_output<T: Into<String>>(self, transaction_id: T, index: u64) -> Self {
        Self {
            transaction: Some(TransactionIdOptions {
                transaction_id: transaction_id.into(),
                output_index: Some(index),
            }),
            ..self
        }
    }

    pub(crate) fn to_url(&self, endpoint: &Url) -> Result<Url, KuponError> {
        if self.address.is_some() && self.credential.is_some() {
            return Err(KuponError::InvalidQuery(
                "cannot query by both address and credential at once".into(),
            ));
        }

        let mut url = endpoint.clone();

        let mut query = url.query_pairs_mut();

        let mut pattern = self.address.clone().or(self.credential.clone());

        if let Some(transaction) = &self.transaction {
            if pattern.is_none() {
                pattern = Some(transaction.to_pattern());
            } else {
                query.append_pair("transaction_id", &transaction.transaction_id);
                if let Some(index) = transaction.output_index {
                    query.append_pair("output_index", &index.to_string());
                }
            }
        }

        if let Some(asset) = &self.asset {
            if pattern.is_none() {
                pattern = Some(asset.to_pattern());
            } else {
                query.append_pair("policy_id", &asset.policy_id);
                if let Some(asset_name) = &asset.asset_name {
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
enum HealthResponse {
    Success(ServerInfo),
    Failure { hint: String },
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
