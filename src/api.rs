//! API layer.
//!
//! A thin HTTP client that talks to a registry's Trusted Publishing endpoints.
//! It knows nothing about CLI arguments or how the token/crate were resolved --
//! it just takes a host + token and exchanges JSON with the registry, mirroring
//! the `Registry` methods in cargo's `crates-io` crate.

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

/// A single GitHub Actions Trusted Publishing config as returned by the registry.
#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubConfig {
    pub id: u32,
    #[serde(rename = "crate")]
    pub krate: String,
    pub repository_owner: String,
    pub repository_owner_id: Option<u32>,
    pub repository_name: String,
    pub workflow_filename: String,
    pub environment: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Deserialize)]
struct GitHubConfigs {
    github_configs: Vec<GitHubConfig>,
}

#[derive(Deserialize)]
struct GitHubConfigResponse {
    github_config: GitHubConfig,
}

#[derive(Serialize)]
struct NewGitHubConfig<'a> {
    #[serde(rename = "crate")]
    krate: &'a str,
    repository_owner: &'a str,
    repository_name: &'a str,
    workflow_filename: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<&'a str>,
}

#[derive(Serialize)]
struct NewGitHubConfigReq<'a> {
    github_config: NewGitHubConfig<'a>,
}

/// A single GitLab CI/CD Trusted Publishing config as returned by the registry.
#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabConfig {
    pub id: u32,
    #[serde(rename = "crate")]
    pub krate: String,
    pub namespace: String,
    pub namespace_id: Option<String>,
    pub project: String,
    pub workflow_filepath: String,
    pub environment: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Deserialize)]
struct GitLabConfigs {
    gitlab_configs: Vec<GitLabConfig>,
}

#[derive(Deserialize)]
struct GitLabConfigResponse {
    gitlab_config: GitLabConfig,
}

#[derive(Serialize)]
struct NewGitLabConfig<'a> {
    #[serde(rename = "crate")]
    krate: &'a str,
    namespace: &'a str,
    project: &'a str,
    workflow_filepath: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<&'a str>,
}

#[derive(Serialize)]
struct NewGitLabConfigReq<'a> {
    gitlab_config: NewGitLabConfig<'a>,
}

#[derive(Serialize)]
struct CrateUpdate {
    trustpub_only: bool,
}

#[derive(Serialize)]
struct CrateUpdateReq {
    #[serde(rename = "crate")]
    krate: CrateUpdate,
}

/// Response of `GET /api/v1/crates/{crate}`; we only need `crate.trustpub_only`.
#[derive(Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    krate: CrateInfo,
}

#[derive(Deserialize)]
struct CrateInfo {
    trustpub_only: bool,
}

/// The error envelope crates.io returns on a non-2xx response:
/// `{"errors":[{"detail":"..."}]}`.
#[derive(Deserialize)]
struct ApiErrorList {
    errors: Vec<ApiError>,
}

#[derive(Deserialize)]
struct ApiError {
    detail: String,
}

/// HTTP client for a registry's Trusted Publishing API.
pub struct Client {
    host: String,
    token: Option<String>,
    agent: ureq::Agent,
}

impl Client {
    /// Creates a client against `host` (e.g. `https://crates.io`). `token` is the
    /// raw registry API token, sent verbatim in the `Authorization` header.
    pub fn new(host: String, token: Option<String>) -> Self {
        // Keep non-2xx responses as `Ok` so `send` can read the registry's error
        // `detail` from the response body instead of losing it to an error variant.
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build();
        Client {
            host,
            token,
            agent: ureq::Agent::new_with_config(config),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1{}", self.host.trim_end_matches('/'), path)
    }

    fn token(&self) -> Result<&str> {
        self.token.as_deref().context(
            "no API token found; set `CARGO_REGISTRY_TOKEN` \
             or run `cargo login` first",
        )
    }

    /// Reads whether the crate is restricted to Trusted Publishing only.
    pub fn trustpub_only(&self, krate: &str) -> Result<bool> {
        let body = send(
            self.agent
                .get(self.url(&format!("/crates/{krate}")))
                .header("Accept", "application/json")
                .header("Authorization", self.token()?)
                .call(),
        )?;
        Ok(serde_json::from_str::<CrateResponse>(&body)?
            .krate
            .trustpub_only)
    }

    pub fn set_trustpub_only(&self, krate: &str, trustpub_only: bool) -> Result<()> {
        let payload = serde_json::to_string(&CrateUpdateReq {
            krate: CrateUpdate { trustpub_only },
        })?;
        send(
            self.agent
                .patch(self.url(&format!("/crates/{krate}")))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", self.token()?)
                .send(payload.as_str()),
        )?;
        Ok(())
    }

    // ----- GitHub -----

    pub fn list_github_configs(&self, krate: &str) -> Result<Vec<GitHubConfig>> {
        let body = send(
            self.agent
                .get(self.url("/trusted_publishing/github_configs"))
                .query("crate", krate)
                .header("Accept", "application/json")
                .header("Authorization", self.token()?)
                .call(),
        )?;
        Ok(serde_json::from_str::<GitHubConfigs>(&body)?.github_configs)
    }

    pub fn add_github_config(
        &self,
        krate: &str,
        repository_owner: &str,
        repository_name: &str,
        workflow_filename: &str,
        environment: Option<&str>,
    ) -> Result<GitHubConfig> {
        let payload = serde_json::to_string(&NewGitHubConfigReq {
            github_config: NewGitHubConfig {
                krate,
                repository_owner,
                repository_name,
                workflow_filename,
                environment,
            },
        })?;
        let body = send(
            self.agent
                .post(self.url("/trusted_publishing/github_configs"))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", self.token()?)
                .send(payload.as_str()),
        )?;
        Ok(serde_json::from_str::<GitHubConfigResponse>(&body)?.github_config)
    }

    pub fn remove_github_config(&self, id: u32) -> Result<()> {
        send(
            self.agent
                .delete(self.url(&format!("/trusted_publishing/github_configs/{id}")))
                .header("Accept", "application/json")
                .header("Authorization", self.token()?)
                .call(),
        )?;
        Ok(())
    }

    // ----- GitLab -----

    pub fn list_gitlab_configs(&self, krate: &str) -> Result<Vec<GitLabConfig>> {
        let body = send(
            self.agent
                .get(self.url("/trusted_publishing/gitlab_configs"))
                .query("crate", krate)
                .header("Accept", "application/json")
                .header("Authorization", self.token()?)
                .call(),
        )?;
        Ok(serde_json::from_str::<GitLabConfigs>(&body)?.gitlab_configs)
    }

    pub fn add_gitlab_config(
        &self,
        krate: &str,
        namespace: &str,
        project: &str,
        workflow_filepath: &str,
        environment: Option<&str>,
    ) -> Result<GitLabConfig> {
        let payload = serde_json::to_string(&NewGitLabConfigReq {
            gitlab_config: NewGitLabConfig {
                krate,
                namespace,
                project,
                workflow_filepath,
                environment,
            },
        })?;
        let body = send(
            self.agent
                .post(self.url("/trusted_publishing/gitlab_configs"))
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", self.token()?)
                .send(payload.as_str()),
        )?;
        Ok(serde_json::from_str::<GitLabConfigResponse>(&body)?.gitlab_config)
    }

    pub fn remove_gitlab_config(&self, id: u32) -> Result<()> {
        send(
            self.agent
                .delete(self.url(&format!("/trusted_publishing/gitlab_configs/{id}")))
                .header("Accept", "application/json")
                .header("Authorization", self.token()?)
                .call(),
        )?;
        Ok(())
    }
}

/// Processes a sent request's result and returns the response body, turning a
/// non-2xx status into an error carrying the registry's `detail` message.
///
/// The agent is configured with `http_status_as_error(false)`, so non-2xx
/// responses arrive here as `Ok` and the status is checked explicitly.
fn send(result: Result<ureq::http::Response<ureq::Body>, ureq::Error>) -> Result<String> {
    let mut response =
        result.map_err(|err| anyhow::Error::new(err).context("failed to reach the registry"))?;
    let status = response.status();
    let body = response
        .body_mut()
        .read_to_string()
        .context("invalid (non-UTF8) response body from registry")?;
    if status.is_success() {
        return Ok(body);
    }
    let detail = serde_json::from_str::<ApiErrorList>(&body)
        .ok()
        .map(|e| {
            e.errors
                .into_iter()
                .map(|x| x.detail)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or(body);
    bail!(
        "the registry responded with an error (status {}): {detail}",
        status.as_u16()
    )
}
