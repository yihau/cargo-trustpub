# cargo-trustpub

[![crates.io](https://img.shields.io/crates/v/cargo-trustpub.svg?cacheSeconds=3600)](https://crates.io/crates/cargo-trustpub)

Manage [Trusted Publishing] configuration for a crate on a registry (crates.io by
default).

![demo](demo.gif)

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

### Example: set up Trusted Publishing for a crate

Authorize the GitHub Actions workflow that publishes the crate, then require all
new versions to go through Trusted Publishing:

```sh
cargo trustpub add --publisher github --owner yihau --repo cargo-trustpub --pipeline release.yml --env Release
cargo trustpub set --trustpub-only true
```

Check the result with `status`:

```sh
cargo trustpub status
```

```text
crate: cargo-trustpub
trustpub_only: enabled
configs:
  10918: github yihau/cargo-trustpub workflow=release.yml environment=Release
```

[Trusted Publishing]: https://crates.io/docs/trusted-publishing
