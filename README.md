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

Requires the `gh` CLI to be installed and authenticated (used for token retrieval).

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

Create `config.yaml` in the current directory or `~/.config/gh-dispatch/config.yaml`:

```yaml
apps:
  my-app:
    build:
      repo: "owner/repo"
      workflow: "build.yml"
      inputs:
        app: "my-app"  # pre-filled, won't prompt
    deploy:
      repo: "owner/repo"
      workflow: "deploy.yml"

  another-app:
    build:
      repo: "owner/other-repo"
      workflow: "ci.yml"
    deploy:
      repo: "owner/other-repo"
      workflow: "deploy.yml"
```

## License

MIT
