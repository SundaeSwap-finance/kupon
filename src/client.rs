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
}

#[derive(Clone, Debug)]
enum SpentStatus {
    Unspent,
    Spent,
}

#[derive(Clone, Debug, Default)]
pub struct MatchOptions<'a> {
    spent_status: Option<SpentStatus>,
    address: Option<&'a str>,
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

    pub fn address<T: Into<&'a str>>(self, address: T) -> Self {
        Self {
            address: Some(address.into()),
            ..self
        }
    }

    pub(crate) fn to_url(&self, endpoint: &Url) -> Result<Url, KuponError> {
        let mut url = endpoint.clone();

        if let Some(address) = self.address {
            url.set_path(&format!("matches/{}", address));
        } else {
            url.set_path("matches");
        }

        let mut query = url.query_pairs_mut();
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
        Ok(url)
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum MatchResponse {
    Success(Vec<Match>),
    Failure { hint: String },
}
