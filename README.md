# cargo-trustpub

[![crates.io](https://img.shields.io/crates/v/cargo-trustpub.svg)](https://crates.io/crates/cargo-trustpub)

Manage [Trusted Publishing] configuration for a crate on a registry (crates.io by
default).

## Install

```sh
cargo install cargo-trustpub
```

## Usage

By default it operates on the package in the current directory and talks to
crates.io; override with `--crate <CRATE>` and `--host <URL>`.

The token is resolved in this order:

1. `CARGO_REGISTRY_TOKEN` environment variable
2. token saved by `cargo login` in `$CARGO_HOME/credentials.toml`

```sh
# either log in once...
cargo login

# ...or pass the token via the environment
export CARGO_REGISTRY_TOKEN=cio...

# show whether Trusted Publishing is required, plus all GitHub + GitLab configs
cargo trustpub status
cargo trustpub status --json   # machine-readable output

# add a config (--publisher selects GitHub or GitLab)
cargo trustpub add --publisher github --owner my-org --repo my-repo --pipeline ci.yml [--env release]
cargo trustpub add --publisher gitlab --owner my-group --repo my-project --pipeline .gitlab-ci.yml

# remove a config (id comes from `status`; --publisher selects the namespace)
cargo trustpub remove --publisher github --id 42

# require Trusted Publishing for new versions
cargo trustpub set --trustpub-only true
```

[Trusted Publishing]: https://crates.io/docs/trusted-publishing
