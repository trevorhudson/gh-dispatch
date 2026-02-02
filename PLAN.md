# gh-dispatch Implementation Plan

Reimplement dispatch-cli functionality using octocrab for direct GitHub API access.

## Current Functionality to Replicate

1. **Config loading** - YAML config mapping apps → build/deploy workflow refs
2. **Fetch workflow schema** - Get workflow file from GitHub, parse `workflow_dispatch.inputs`
3. **Interactive prompts** - Dynamic prompts based on input schema (choice/boolean/string)
4. **Trigger workflow** - Dispatch with collected inputs
5. **CLI** - App selection, workflow selection, confirmation

## Module Structure

```
src/
├── main.rs      # CLI entry point (clap)
├── config.rs    # Config loading (can largely copy from current)
├── github.rs    # Octocrab client, workflow fetching, dispatching
└── prompts.rs   # Interactive prompts (can largely copy from current)
```

## [x] Phase 1: Dependencies & Auth

Add to `Cargo.toml`:

```toml
[dependencies]
octocrab = "0.44"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
indexmap = { version = "2", features = ["serde"] }
inquire = "0.7"
```

For auth, octocrab can use:
- `GITHUB_TOKEN` env var
- Or shell out to `gh auth token` as fallback

## Phase 2: GitHub Module with Octocrab

Key octocrab calls:

```rust
// Get workflow file content
octocrab.repos(owner, repo)
    .get_content()
    .path(".github/workflows/build.yml")
    .send()
    .await?

// Trigger workflow dispatch
octocrab.actions()
    .create_workflow_dispatch(owner, repo, workflow_id, ref)
    .inputs(serde_json::json!({ ... }))
    .send()
    .await?
```

## Phase 3: Config & Prompts

These can be mostly copied from dispatch-cli - they don't depend on `gh`.

## Phase 4: Wire Up Main

Same flow, but `main` becomes `async fn main()` with tokio runtime.

## Implementation Order

- [ ] Set up `Cargo.toml` with dependencies
- [ ] Get octocrab auth working (instantiate client, make a test call)
- [ ] Port `config.rs` (straightforward copy)
- [ ] Implement `github.rs` with octocrab (fetch schema, trigger dispatch)
- [ ] Port `prompts.rs` (straightforward copy)
- [ ] Wire up `main.rs`

## Future Enhancements (post-parity)

- Poll workflow run status after dispatch
- Chain build → deploy workflows
- Stream workflow logs
