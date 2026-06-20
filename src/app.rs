//! App layer.
//!
//! Parses CLI arguments, resolves the crate name and API token, then drives the
//! [`api::Client`] and renders results. This mirrors the responsibilities of
//! cargo's `ops/registry/trustpub.rs` plus `bin/cargo/commands/trustpub.rs`,
//! but as a standalone external `cargo trustpub` subcommand.

use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::api::Client;

/// Default registry host when `--host` is not given.
const DEFAULT_HOST: &str = "https://crates.io";

/// Which Trusted Publishing provider a config belongs to.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Publisher {
    Github,
    Gitlab,
}

/// `cargo trustpub` is invoked by cargo as `cargo-trustpub trustpub ...`, so the
/// top-level parser is an enum with a single `Trustpub` variant.
#[derive(Parser)]
#[command(name = "cargo-trustpub", bin_name = "cargo")]
enum Cargo {
    /// Manage Trusted Publishing configuration for a crate on the registry
    Trustpub(TrustpubArgs),
}

#[derive(Args)]
struct TrustpubArgs {
    /// Crate to operate on (defaults to the package in the current directory)
    #[arg(long = "crate", value_name = "CRATE", global = true)]
    krate: Option<String>,

    /// Registry host to talk to
    #[arg(long, value_name = "URL", global = true, default_value = DEFAULT_HOST)]
    host: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show Trusted Publishing status: whether it is required, and all configs
    #[command(alias = "list")]
    Status {
        /// Output as JSON instead of human-readable text
        #[arg(long)]
        json: bool,
    },

    /// Add a Trusted Publishing config to a crate
    Add {
        /// Trusted Publishing provider
        #[arg(long, value_name = "PUBLISHER")]
        publisher: Publisher,
        /// Repository owner (GitHub) or namespace (GitLab)
        #[arg(long, value_name = "OWNER")]
        owner: String,
        /// Repository name (GitHub) or project (GitLab)
        #[arg(long, value_name = "REPO")]
        repo: String,
        /// Workflow file: filename for GitHub (e.g. `ci.yml`), filepath for GitLab (e.g. `.gitlab-ci.yml`)
        #[arg(long, value_name = "PIPELINE")]
        pipeline: String,
        /// Environment the workflow must run in
        #[arg(long, value_name = "ENV")]
        env: Option<String>,
    },

    /// Remove a Trusted Publishing config from a crate
    Remove {
        /// Trusted Publishing provider the config belongs to
        #[arg(long, value_name = "PUBLISHER")]
        publisher: Publisher,
        /// Id of the config to remove (see `cargo trustpub status`)
        #[arg(long, value_name = "ID")]
        id: u32,
    },

    /// Control whether new versions must be published via Trusted Publishing
    Set {
        /// Whether new versions must be published via Trusted Publishing (`true` or `false`)
        #[arg(long = "trustpub-only", value_name = "BOOL", action = clap::ArgAction::Set)]
        trustpub_only: bool,
    },
}

pub fn run() -> Result<()> {
    let Cargo::Trustpub(args) = Cargo::parse();

    let krate = resolve_crate(args.krate)?;
    let token = resolve_token();
    let client = Client::new(args.host, token);

    match args.command {
        Command::Status { json } => status(&client, &krate, json),
        Command::Add {
            publisher,
            owner,
            repo,
            pipeline,
            env,
        } => add(
            &client,
            &krate,
            publisher,
            &owner,
            &repo,
            &pipeline,
            env.as_deref(),
        ),
        Command::Remove { publisher, id } => remove(&client, &krate, publisher, id),
        Command::Set { trustpub_only } => set(&client, &krate, trustpub_only),
    }
}

fn status(client: &Client, krate: &str, json: bool) -> Result<()> {
    let on_registry = |what: &str| {
        format!(
            "failed to {what} for crate `{krate}` on registry at {}",
            client.host()
        )
    };

    let trustpub_only = client
        .trustpub_only(krate)
        .with_context(|| on_registry("read Trusted Publishing status"))?;
    let github = client
        .list_github_configs(krate)
        .with_context(|| on_registry("list GitHub trusted publishing configs"))?;
    let gitlab = client
        .list_gitlab_configs(krate)
        .with_context(|| on_registry("list GitLab trusted publishing configs"))?;

    if json {
        let mut configs = Vec::with_capacity(github.len() + gitlab.len());
        for c in &github {
            configs.push(tagged_config("github", c)?);
        }
        for c in &gitlab {
            configs.push(tagged_config("gitlab", c)?);
        }
        let out = serde_json::json!({
            "crate": krate,
            "trustpub_only": trustpub_only,
            "configs": configs,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("crate: {krate}");
    println!(
        "trustpub_only: {}",
        if trustpub_only { "enabled" } else { "disabled" }
    );

    if github.is_empty() && gitlab.is_empty() {
        println!("configs: none");
        return Ok(());
    }

    println!("configs:");
    for c in &github {
        println!(
            "  {}: github {}/{} workflow={}{}",
            c.id,
            c.repository_owner,
            c.repository_name,
            c.workflow_filename,
            fmt_env(c.environment.as_deref()),
        );
    }
    for c in &gitlab {
        println!(
            "  {}: gitlab {}/{} workflow={}{}",
            c.id,
            c.namespace,
            c.project,
            c.workflow_filepath,
            fmt_env(c.environment.as_deref()),
        );
    }
    Ok(())
}

fn add(
    client: &Client,
    krate: &str,
    publisher: Publisher,
    owner: &str,
    repo: &str,
    pipeline: &str,
    environment: Option<&str>,
) -> Result<()> {
    let ctx = || {
        format!(
            "failed to add trusted publishing config to crate `{krate}` on registry at {}",
            client.host()
        )
    };

    let (id, summary) = match publisher {
        Publisher::Github => {
            let c = client
                .add_github_config(krate, owner, repo, pipeline, environment)
                .with_context(ctx)?;
            (
                c.id,
                format!(
                    "github {}/{} workflow={}{}",
                    c.repository_owner,
                    c.repository_name,
                    c.workflow_filename,
                    fmt_env(c.environment.as_deref()),
                ),
            )
        }
        Publisher::Gitlab => {
            let c = client
                .add_gitlab_config(krate, owner, repo, pipeline, environment)
                .with_context(ctx)?;
            (
                c.id,
                format!(
                    "gitlab {}/{} workflow={}{}",
                    c.namespace,
                    c.project,
                    c.workflow_filepath,
                    fmt_env(c.environment.as_deref()),
                ),
            )
        }
    };

    println!("Added trusted publishing config {id} ({summary}) for crate `{krate}`");
    Ok(())
}

fn remove(client: &Client, krate: &str, publisher: Publisher, id: u32) -> Result<()> {
    let ctx = || {
        format!(
            "failed to remove trusted publishing config {id} from crate `{krate}` on registry at {}",
            client.host()
        )
    };
    match publisher {
        Publisher::Github => client.remove_github_config(id).with_context(ctx)?,
        Publisher::Gitlab => client.remove_gitlab_config(id).with_context(ctx)?,
    }
    println!("Removed trusted publishing config {id} for crate `{krate}`");
    Ok(())
}

fn set(client: &Client, krate: &str, trustpub_only: bool) -> Result<()> {
    client
        .set_trustpub_only(krate, trustpub_only)
        .with_context(|| {
            format!(
                "failed to update `trustpub_only` for crate `{krate}` on registry at {}",
                client.host()
            )
        })?;
    println!("Updated `trustpub_only` for crate `{krate}` to {trustpub_only}");
    Ok(())
}

/// Serializes a config to JSON and tags it with its `publisher`, so GitHub and
/// GitLab configs can live in one aggregated array.
fn tagged_config<T: serde::Serialize>(publisher: &str, config: &T) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(config)?;
    let object = value
        .as_object_mut()
        .context("expected config to serialize to a JSON object")?;
    object.insert("publisher".to_string(), serde_json::json!(publisher));
    Ok(value)
}

fn fmt_env(environment: Option<&str>) -> String {
    match environment {
        Some(env) => format!(" environment={env}"),
        None => String::new(),
    }
}

/// Uses the explicit `--crate` value if given, otherwise reads the package name
/// from `Cargo.toml` in the current directory.
fn resolve_crate(krate: Option<String>) -> Result<String> {
    if let Some(krate) = krate {
        return Ok(krate);
    }

    let manifest = fs::read_to_string("Cargo.toml").context(
        "no `--crate` given and no `Cargo.toml` found in the current directory; \
         pass `--crate <CRATE>`",
    )?;
    let manifest: toml::Table = manifest.parse().context("failed to parse `Cargo.toml`")?;
    manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string())
        .context("could not find `package.name` in `Cargo.toml`; pass `--crate <CRATE>`")
}

/// Resolves the API token, preferring the `CARGO_REGISTRY_TOKEN` env var, then
/// `[registry].token` in the cargo credentials file. Returns `None` if nothing
/// is configured.
fn resolve_token() -> Option<String> {
    if let Ok(token) = env::var("CARGO_REGISTRY_TOKEN")
        && !token.is_empty()
    {
        return Some(token);
    }
    token_from_credentials()
}

fn token_from_credentials() -> Option<String> {
    let cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")))?;

    // `credentials.toml` is the current name; `credentials` is the legacy one.
    for name in ["credentials.toml", "credentials"] {
        let path = cargo_home.join(name);
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(value) = contents.parse::<toml::Table>()
            && let Some(token) = value
                .get("registry")
                .and_then(|r| r.get("token"))
                .and_then(|t| t.as_str())
        {
            return Some(token.to_string());
        }
    }
    None
}
