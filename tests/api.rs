//! Integration tests for the API layer.
//!
//! Each test starts a local mock HTTP server, points a `Client` at it, and
//! asserts both the request we send (method, path, query, auth header, body)
//! and that we decode the response correctly.

use cargo_trustpub::api::Client;
use httpmock::prelude::*;
use serde_json::json;

const TOKEN: &str = "test-token";

fn client(server: &MockServer) -> Client {
    Client::new(server.base_url(), Some(TOKEN.to_string()))
}

#[test]
fn lists_github_configs() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/trusted_publishing/github_configs")
            .query_param("crate", "regex")
            .header("authorization", TOKEN);
        then.status(200).json_body(json!({
            "github_configs": [{
                "id": 42,
                "crate": "regex",
                "repository_owner": "rust-lang",
                "repository_owner_id": 5430905,
                "repository_name": "regex",
                "workflow_filename": "ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }],
            "meta": { "total": 1 }
        }));
    });

    let configs = client(&server).list_github_configs("regex").unwrap();

    mock.assert();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].id, 42);
    assert_eq!(configs[0].repository_owner, "rust-lang");
    assert_eq!(configs[0].workflow_filename, "ci.yml");
}

#[test]
fn adds_github_config() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/trusted_publishing/github_configs")
            .header("authorization", TOKEN)
            .json_body(json!({
                "github_config": {
                    "crate": "regex",
                    "repository_owner": "rust-lang",
                    "repository_name": "regex",
                    "workflow_filename": "ci.yml",
                    "environment": "release"
                }
            }));
        then.status(200).json_body(json!({
            "github_config": {
                "id": 7,
                "crate": "regex",
                "repository_owner": "rust-lang",
                "repository_owner_id": 5430905,
                "repository_name": "regex",
                "workflow_filename": "ci.yml",
                "environment": "release",
                "created_at": "2026-01-01T00:00:00Z"
            }
        }));
    });

    let config = client(&server)
        .add_github_config("regex", "rust-lang", "regex", "ci.yml", Some("release"))
        .unwrap();

    mock.assert();
    assert_eq!(config.id, 7);
    assert_eq!(config.environment.as_deref(), Some("release"));
}

#[test]
fn omits_environment_when_absent() {
    let server = MockServer::start();
    // The request body must NOT contain an `environment` key when None.
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/trusted_publishing/github_configs")
            .json_body(json!({
                "github_config": {
                    "crate": "regex",
                    "repository_owner": "rust-lang",
                    "repository_name": "regex",
                    "workflow_filename": "ci.yml"
                }
            }));
        then.status(200).json_body(json!({
            "github_config": {
                "id": 1,
                "crate": "regex",
                "repository_owner": "rust-lang",
                "repository_owner_id": null,
                "repository_name": "regex",
                "workflow_filename": "ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }
        }));
    });

    client(&server)
        .add_github_config("regex", "rust-lang", "regex", "ci.yml", None)
        .unwrap();

    mock.assert();
}

#[test]
fn removes_github_config() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(DELETE)
            .path("/api/v1/trusted_publishing/github_configs/42")
            .header("authorization", TOKEN);
        then.status(200).json_body(json!({ "ok": true }));
    });

    client(&server).remove_github_config(42).unwrap();
    mock.assert();
}

#[test]
fn lists_gitlab_configs() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/trusted_publishing/gitlab_configs")
            .query_param("crate", "regex");
        then.status(200).json_body(json!({
            "gitlab_configs": [{
                "id": 9,
                "crate": "regex",
                "namespace": "my-group",
                "namespace_id": null,
                "project": "my-project",
                "workflow_filepath": ".gitlab-ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }],
            "meta": { "total": 1 }
        }));
    });

    let configs = client(&server).list_gitlab_configs("regex").unwrap();

    mock.assert();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].namespace, "my-group");
    assert_eq!(configs[0].project, "my-project");
    assert_eq!(configs[0].workflow_filepath, ".gitlab-ci.yml");
}

#[test]
fn adds_gitlab_config() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/trusted_publishing/gitlab_configs")
            .json_body(json!({
                "gitlab_config": {
                    "crate": "regex",
                    "namespace": "my-group",
                    "project": "my-project",
                    "workflow_filepath": ".gitlab-ci.yml"
                }
            }));
        then.status(200).json_body(json!({
            "gitlab_config": {
                "id": 11,
                "crate": "regex",
                "namespace": "my-group",
                "namespace_id": null,
                "project": "my-project",
                "workflow_filepath": ".gitlab-ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }
        }));
    });

    let config = client(&server)
        .add_gitlab_config("regex", "my-group", "my-project", ".gitlab-ci.yml", None)
        .unwrap();

    mock.assert();
    assert_eq!(config.id, 11);
}

#[test]
fn reads_trustpub_only() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/api/v1/crates/regex");
        then.status(200)
            .json_body(json!({ "crate": { "trustpub_only": true } }));
    });

    let only = client(&server).trustpub_only("regex").unwrap();
    mock.assert();
    assert!(only);
}

#[test]
fn sets_trustpub_only() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(httpmock::Method::PATCH)
            .path("/api/v1/crates/regex")
            .header("authorization", TOKEN)
            .json_body(json!({ "crate": { "trustpub_only": true } }));
        then.status(200)
            .json_body(json!({ "crate": { "trustpub_only": true } }));
    });

    client(&server).set_trustpub_only("regex", true).unwrap();
    mock.assert();
}

#[test]
fn surfaces_api_error_detail() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/trusted_publishing/github_configs");
        then.status(403)
            .json_body(json!({ "errors": [{ "detail": "must be an owner" }] }));
    });

    let err = client(&server)
        .list_github_configs("regex")
        .unwrap_err()
        .to_string();
    assert!(err.contains("must be an owner"), "got: {err}");
    assert!(err.contains("403"), "got: {err}");
}

#[test]
fn missing_token_errors_before_request() {
    let server = MockServer::start();
    // No mock registered: if a request were sent it would 404, but token()
    // should fail first, so nothing reaches the server.
    let client = Client::new(server.base_url(), None);
    let err = client.list_github_configs("regex").unwrap_err().to_string();
    assert!(err.contains("no API token"), "got: {err}");
}
