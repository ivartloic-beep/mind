//! Client HTTP mind-api.

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::config::SyncConfig;

#[derive(Debug, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}

#[derive(Clone)]
pub struct MindClient {
    http: reqwest::Client,
    base_url: String,
    token: String,
}

impl MindClient {
    pub fn from_config(cfg: &SyncConfig) -> Result<Self, String> {
        if cfg.token.trim().is_empty() {
            return Err("token API manquant".into());
        }
        let base = cfg.base_url.trim().trim_end_matches('/').to_string();
        if base.is_empty() {
            return Err("URL API manquante".into());
        }
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            http,
            base_url: base,
            token: cfg.token.trim().to_string(),
        })
    }

    pub async fn health(&self) -> Result<Value, String> {
        let url = format!("{}/health", self.base_url);
        let res = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            return Err(format!("health HTTP {}", res.status()));
        }
        res.json().await.map_err(|e| e.to_string())
    }

    pub async fn list<T: DeserializeOwned>(
        &self,
        entity: &str,
        since: Option<&str>,
    ) -> Result<Vec<T>, String> {
        let url = format!("{}/{}", self.base_url, entity);
        let mut req = self
            .http
            .get(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .query(&[("limit", "1000")]);
        if let Some(s) = since {
            req = req.query(&[("since", s)]);
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("unauthorized".into());
        }
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(format!("list {entity}: HTTP {status} {body}"));
        }
        let parsed: ListResponse<T> = res.json().await.map_err(|e| e.to_string())?;
        Ok(parsed.items)
    }

    pub async fn put<T: Serialize>(&self, entity: &str, id: &str, body: &T) -> Result<(), String> {
        let url = format!("{}/{}/{}", self.base_url, entity, id);
        let res = self
            .http
            .put(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header(CONTENT_TYPE, "application/json")
            .json(body)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("unauthorized".into());
        }
        if !status.is_success() {
            let body = res.text().await.unwrap_or_default();
            return Err(format!("put {entity}/{id}: HTTP {status} {body}"));
        }
        Ok(())
    }

    pub async fn delete(&self, entity: &str, id: &str) -> Result<(), String> {
        let url = format!("{}/{}/{}", self.base_url, entity, id);
        let res = self
            .http
            .delete(&url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = res.status();
        if status.as_u16() == 401 {
            return Err("unauthorized".into());
        }
        // 404 = déjà absent → OK (idempotent)
        if status.as_u16() == 404 || status.is_success() {
            return Ok(());
        }
        let body = res.text().await.unwrap_or_default();
        Err(format!("delete {entity}/{id}: HTTP {status} {body}"))
    }
}
