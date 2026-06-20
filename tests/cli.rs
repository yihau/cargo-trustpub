//! End-to-end tests for the app layer: run the built binary against a mock
//! registry and assert on exit status and rendered output.

use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use serde_json::json;

const TOKEN: &str = "test-token";

fn cargo_trustpub() -> Command {
    let mut cmd = Command::cargo_bin("cargo-trustpub").unwrap();
    // Don't pick up a developer's real token from the environment.
    cmd.env_remove("CARGO_REGISTRY_TOKEN");
    cmd
}

/// Registers the three calls `status` makes (trustpub_only + both config lists).
fn mock_status(server: &MockServer, trustpub_only: bool) {
    server.mock(|when, then| {
        when.method(GET).path("/api/v1/crates/cargo-trustpub");
        then.status(200)
            .json_body(json!({ "crate": { "trustpub_only": trustpub_only } }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/trusted_publishing/github_configs");
        then.status(200).json_body(json!({
            "github_configs": [{
                "id": 42,
                "crate": "cargo-trustpub",
                "repository_owner": "yihau",
                "repository_owner_id": 5430905,
                "repository_name": "cargo-trustpub",
                "workflow_filename": "ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }],
            "meta": { "total": 1 }
        }));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/api/v1/trusted_publishing/gitlab_configs");
        then.status(200).json_body(json!({
            "gitlab_configs": [{
                "id": 9,
                "crate": "cargo-trustpub",
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
}

#[test]
fn status_human_output() {
    let server = MockServer::start();
    mock_status(&server, true);

    cargo_trustpub()
        .env("CARGO_REGISTRY_TOKEN", TOKEN)
        .args([
            "trustpub",
            "status",
            "--crate",
            "cargo-trustpub",
            "--host",
            &server.base_url(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("trustpub_only: enabled"))
        .stdout(predicate::str::contains(
            "42: github yihau/cargo-trustpub workflow=ci.yml",
        ))
        .stdout(predicate::str::contains(
            "9: gitlab my-group/my-project workflow=.gitlab-ci.yml",
        ));
}

#[test]
fn status_json_is_aggregated_and_tagged() {
    let server = MockServer::start();
    mock_status(&server, false);

    let assert = cargo_trustpub()
        .env("CARGO_REGISTRY_TOKEN", TOKEN)
        .args([
            "trustpub",
            "status",
            "--crate",
            "cargo-trustpub",
            "--host",
            &server.base_url(),
            "--json",
        ])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["crate"], "cargo-trustpub");
    assert_eq!(value["trustpub_only"], false);
    let configs = value["configs"].as_array().unwrap();
    assert_eq!(configs.len(), 2);
    // Aggregated: GitHub first, then GitLab, each tagged with its publisher.
    assert_eq!(configs[0]["publisher"], "github");
    assert_eq!(configs[0]["repository_owner"], "yihau");
    assert_eq!(configs[1]["publisher"], "gitlab");
    assert_eq!(configs[1]["namespace"], "my-group");
}

#[test]
fn status_token_from_env() {
    let server = MockServer::start();
    mock_status(&server, true);

    // The token comes from CARGO_REGISTRY_TOKEN.
    cargo_trustpub()
        .env("CARGO_REGISTRY_TOKEN", TOKEN)
        .args([
            "trustpub",
            "status",
            "--crate",
            "cargo-trustpub",
            "--host",
            &server.base_url(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("trustpub_only: enabled"));
}

#[test]
fn add_github_reports_added() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/api/v1/trusted_publishing/github_configs");
        then.status(200).json_body(json!({
            "github_config": {
                "id": 7,
                "crate": "cargo-trustpub",
                "repository_owner": "yihau",
                "repository_owner_id": 5430905,
                "repository_name": "cargo-trustpub",
                "workflow_filename": "ci.yml",
                "environment": null,
                "created_at": "2026-01-01T00:00:00Z"
            }
        }));
    });

    cargo_trustpub()
        .env("CARGO_REGISTRY_TOKEN", TOKEN)
        .args([
            "trustpub",
            "add",
            "--publisher",
            "github",
            "--owner",
            "yihau",
            "--repo",
            "cargo-trustpub",
            "--pipeline",
            "ci.yml",
            "--crate",
            "cargo-trustpub",
            "--host",
            &server.base_url(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added trusted publishing config 7",
        ));
    mock.assert();
}

#[test]
fn api_error_is_reported_and_nonzero_exit() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/api/v1/crates/cargo-trustpub");
        then.status(403)
            .json_body(json!({ "errors": [{ "detail": "must be an owner" }] }));
    });

    cargo_trustpub()
        .env("CARGO_REGISTRY_TOKEN", TOKEN)
        .args([
            "trustpub",
            "status",
            "--crate",
            "cargo-trustpub",
            "--host",
            &server.base_url(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("must be an owner"));
}

#[test]
fn invalid_publisher_is_rejected() {
    // clap should reject an unknown --publisher value before any network call.
    cargo_trustpub()
        .args([
            "trustpub",
            "add",
            "--publisher",
            "bitbucket",
            "--owner",
            "o",
            "--repo",
            "r",
            "--pipeline",
            "ci.yml",
            "--crate",
            "cargo-trustpub",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'bitbucket'"));
}
