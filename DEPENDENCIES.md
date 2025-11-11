# Dependency Management

## Current Dependency Issues

### Critical: Local Path Dependencies

The project currently uses local path dependencies for grammers libraries:

```toml
grammers-client = { path = "../grammers-master/grammers-client", features = ["proxy"] }
grammers-session = { path = "../grammers-master/grammers-session" }
grammers-mtsender = { path = "../grammers-master/grammers-mtsender", features = ["proxy"] }
```

**Problem**: These paths are not portable and won't work on other machines.

**Solutions**:

### Option 1: Use Git Dependencies (Recommended)

Replace local paths with git dependencies:

```toml
grammers-client = { git = "https://github.com/Lonami/grammers", features = ["proxy"] }
grammers-session = { git = "https://github.com/Lonami/grammers" }
grammers-mtsender = { git = "https://github.com/Lonami/grammers", features = ["proxy"] }
```

### Option 2: Use crates.io (If Available)

Check if grammers is published on crates.io:

```toml
grammers-client = { version = "0.x", features = ["proxy"] }
grammers-session = "0.x"
grammers-mtsender = { version = "0.x", features = ["proxy"] }
```

### Option 3: Keep Local Paths (Development Only)

If you're developing with a local grammers clone:

1. Ensure `../grammers-master/` exists relative to this project
2. Clone grammers: `cd .. && git clone https://github.com/Lonami/grammers grammers-master`

## Generating Cargo.lock

Once dependencies are fixed:

```bash
cargo update
cargo build
```

This will create `Cargo.lock` which should be committed to git.

## Dependency Versions

Current dependencies (from Cargo.toml):

| Dependency | Version | Purpose |
|------------|---------|---------|
| tokio | 1.x | Async runtime |
| log | 0.4 | Logging facade |
| simple_logger | 4.0 | Basic logger |
| sqlite | 0.37 | SQLite database |
| phoenix_channels_client | 0.1 | Phoenix Channel integration |
| serde | 1.0 | Serialization |
| serde_json | 1.0 | JSON support |
| thiserror | 1.0 | Error handling |
| fs2 | 0.4 | File locking |

## Recommended Updates

Consider upgrading:

1. **sqlite** → Use `sqlx` or `tokio-rusqlite` for async support
2. **simple_logger** → Use `tracing` + `tracing-subscriber` for structured logging
3. **phoenix_channels_client** → Check for newer versions

## Adding Cargo.lock to Git

Once generated, commit it:

```bash
git add Cargo.lock
git commit -m "Add Cargo.lock for reproducible builds"
```

## Next Steps

1. Fix grammers dependencies (choose Option 1, 2, or 3 above)
2. Run `cargo update` to generate Cargo.lock
3. Commit Cargo.lock to version control
4. Update CI/CD to use locked versions
