use thiserror::Error;

#[derive(Error, Debug)]
pub enum KuponError {
    #[error("invalid url")]
    InvalidUrl(#[from] url::ParseError),
    #[error("error from kupo: {0}")]
    KupoError(String),
    #[error("request failed")]
    RequestFailed(#[from] reqwest::Error),
}
