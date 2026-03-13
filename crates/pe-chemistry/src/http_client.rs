//! Abstraction over HTTP transport for testability.
//!
//! In production, `ReqwestHttpClient` wraps `reqwest::Client`.
//! In tests, `MockHttpClient` returns canned responses.

use async_trait::async_trait;
#[cfg(feature = "native")]
use std::time::Duration;

use crate::error::ChemistryError;

/// Minimal HTTP client interface — just enough for the sidecar bridge.
#[async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str) -> Result<HttpResponse, ChemistryError>;
    async fn post(&self, url: &str, json_body: &str) -> Result<HttpResponse, ChemistryError>;
}

/// A simplified HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

/// Production HTTP client backed by reqwest.
#[cfg(feature = "native")]
pub struct ReqwestHttpClient {
    client: reqwest::Client,
}

#[cfg(feature = "native")]
impl ReqwestHttpClient {
    pub fn new(timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("reqwest client build should not fail with these settings");
        Self { client }
    }
}

#[cfg(feature = "native")]
#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn get(&self, url: &str) -> Result<HttpResponse, ChemistryError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ChemistryError::Timeout(Duration::from_secs(30))
                } else {
                    ChemistryError::HttpError(e.to_string())
                }
            })?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| ChemistryError::HttpError(e.to_string()))?;

        Ok(HttpResponse { status, body })
    }

    async fn post(&self, url: &str, json_body: &str) -> Result<HttpResponse, ChemistryError> {
        let resp = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .body(json_body.to_string())
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    ChemistryError::Timeout(Duration::from_secs(30))
                } else {
                    ChemistryError::HttpError(e.to_string())
                }
            })?;

        let status = resp.status().as_u16();
        let body = resp
            .text()
            .await
            .map_err(|e| ChemistryError::HttpError(e.to_string()))?;

        Ok(HttpResponse { status, body })
    }
}
