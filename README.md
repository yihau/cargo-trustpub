# cargo-trustpub

A standalone `cargo` subcommand to manage [Trusted Publishing] configuration for
a crate on a registry (crates.io by default).

## Install

```sh
cargo install --path .
```

Then invoke it through cargo:

```sh
# Show whether Trusted Publishing is required, plus all GitHub + GitLab configs
cargo trustpub status
cargo trustpub status --json   # machine-readable output

# Add a config (--publisher selects GitHub or GitLab)
cargo trustpub add --publisher github --owner my-org --repo my-repo --pipeline ci.yml [--env release]
cargo trustpub add --publisher gitlab --owner my-group --repo my-project --pipeline .gitlab-ci.yml

# Remove a config (id comes from `status`; --publisher selects the namespace)
cargo trustpub remove --publisher github --id 42

# Require Trusted Publishing for new versions
cargo trustpub set --trustpub-only true
```

For `add`/`remove`, `--publisher` chooses the provider. The shared `--owner` /
`--repo` / `--pipeline` flags map to GitHub's `repository_owner` /
`repository_name` / `workflow_filename` and to GitLab's `namespace` / `project`
/ `workflow_filepath`.

Global options: `--crate <CRATE>` (defaults to the package in the current
directory), `--host <URL>` (defaults to `https://crates.io`).

## Authentication

The API token is resolved in this order:

1. the `CARGO_REGISTRY_TOKEN` environment variable (recommended for CI)
2. `[registry].token` from `$CARGO_HOME/credentials.toml` (written by `cargo login`)

## Architecture

The crate is split into two layers:

- **API layer** (`src/api.rs`) — a `Client` that exchanges JSON with the
  registry's Trusted Publishing HTTP endpoints. It is unaware of CLI arguments;
  it only takes a host + token. This mirrors the `Registry` type in cargo's
  `crates-io` crate.
- **App layer** (`src/app.rs`) — parses CLI arguments, resolves the crate name
  and token, drives the API layer, and renders results. This mirrors cargo's
  `ops/registry/trustpub.rs` plus `bin/cargo/commands/trustpub.rs`.

`src/main.rs` is a thin entry point that runs the app layer and reports errors.

[Trusted Publishing]: https://crates.io/docs/trusted-publishing
