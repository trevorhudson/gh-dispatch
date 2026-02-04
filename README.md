# gh-dispatch

A CLI tool for triggering GitHub Actions workflows with interactive prompts and completion polling.

## Features

- Interactive app and workflow selection
- Auto-discovers workflow inputs from GitHub
- Pre-fill inputs via config file
- Polls for workflow completion with live status

## Installation

```bash
cargo install --path .
```

Requires a GitHub token: set `GITHUB_TOKEN`, or have the `gh` CLI installed and authenticated as a fallback.

## Usage

```bash
# Interactive mode - prompts for app and workflow
gh-dispatch

# Specify app
gh-dispatch my-app

# Specify app and workflow
gh-dispatch my-app -w build

# Fire and forget (don't wait for completion)
gh-dispatch my-app -w deploy --no-wait
```

## Configuration

Create `config.toml` in the current directory or `~/.config/gh-dispatch/config.toml`:

```toml
[apps.my-app]
build = { repo = "owner/repo", workflow = "build.yml", inputs = { app = "my-app" } }
deploy = { repo = "owner/repo", workflow = "deploy.yml", ref = "develop", inputs = { app = "my-app", tag = "v1.0" } }
test = { repo = "owner/repo", workflow = "test.yml" }

[apps.another-app]
build = { repo = "owner/other-repo", workflow = "ci.yml" }
deploy = { repo = "owner/other-repo", workflow = "deploy.yml" }
```

The optional `ref` field pins a workflow to a specific branch or tag.  When omitted the repository's default branch is used.

## Using as a `gh` CLI Extension

Because the binary is already named `gh-dispatch`, the `gh` CLI will pick it up as an extension automatically — no code changes required.  After building, place it where `gh` can find it:

```bash
cargo build --release
mkdir -p ~/.local/share/gh/extensions/gh-dispatch
cp target/release/gh-dispatch ~/.local/share/gh/extensions/gh-dispatch/
```

You can then run it as `gh dispatch` with all the same flags:

```bash
gh dispatch my-app -w build
```

> **Note:** `gh extension install <repo>` may also work directly against this repository if the release assets match gh's naming expectations.  If you run into issues, the manual copy above is the reliable fallback.

## License

[MIT](LICENSE.MD)
