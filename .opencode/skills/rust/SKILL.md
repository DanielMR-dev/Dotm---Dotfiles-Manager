---
name: rust-standards
description: Shared Rust development standards for all agents. Covers project conventions, mandatory crates, error handling rules, code quality gates, safety rules, testing standards, and output/UX conventions. Always apply this skill when working on any Rust project.
compatibility: opencode
---

## Language and environment

- **Language**: Rust (stable toolchain, MSRV 1.75+)
- **Target platform**: Arch Linux (primary), other Linux distributions (secondary), macOS (tertiary)
- **Build tool**: Cargo — never use custom build scripts unless strictly necessary
- **Edition**: 2021

---

## Project conventions

### Naming rules
```
snake_case          → files, modules, functions, variables
PascalCase          → types, traits, enums
SCREAMING_SNAKE_CASE → constants and statics
```

### Standard directory layout
```
src/
  main.rs          ← entry point and CLI dispatch only
  cli.rs           ← clap command definitions
  config.rs        ← config deserialization (serde + toml)
  error.rs         ← ALL error types (thiserror)
  manager.rs       ← high-level command logic
  linker.rs        ← symlink / copy engine
  backup.rs        ← backup and restore logic
  source/
    mod.rs         ← Source trait definition
    local.rs       ← local folder source implementation
    github.rs      ← git clone/pull source implementation
tests/
  integration.rs   ← end-to-end tests via assert_cmd
```
 
### Module rules
- One module per file — never define multiple modules in the same file
- `mod.rs` contains only re-exports and trait/type definitions, never business logic
- `pub` surface must be minimal — expose only what callers actually need
- Use `pub(crate)` for internal shared types

---

## Mandatory crates

| Purpose | Crate | Notes |
|---|---|---|
| CLI | `clap` (derive feature) | Always use derive macros, not builder API |
| Config parsing | `serde` + `toml` | All config structs derive `Deserialize, Serialize` |
| Error types (lib) | `thiserror` | One `Error` enum per module that needs it |
| Error propagation (bin) | `anyhow` | Only in `main.rs` and CLI dispatch |
| Filesystem paths | `dirs` | Never hardcode `~` — always resolve via `dirs::home_dir()` |
| Terminal output | `colored` | All status messages use color helpers |
| Progress indicators | `indicatif` | Long operations (clone, sync) always show a spinner |
| Git operations | `git2` | Feature-gated as `git` |

Do not add any crate to `Cargo.toml` without justification. If a new dependency is needed, state the reason explicitly.

---

## Error handling rules

```rust
// CORRECT — carries full context
#[derive(Debug, Error)]
pub enum DotmError {
    #[error("IO error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
 
// WRONG — loses context, never do this
#[derive(Debug, Error)]
pub enum DotmError {
    #[error("IO error")]
    Io(#[from] std::io::Error),
}
```
 
### Rules
- Every error must carry context — what operation was being performed, not just what failed
- `thiserror` is mandatory for all custom error types
- `.unwrap()` is **forbidden** in all non-test code — use `?`, `.ok_or()`, or explicit `match`
- `.expect("msg")` is allowed only when failure is genuinely impossible — the message must explain the invariant
- Error enums must be defined in `error.rs` before any module that uses them
- Always preserve the error chain with `#[source]` or `#[from]`

---

## Code quality gates

All code produced or modified must pass these checks before being considered complete:

```bash
cargo check               # must compile with zero errors
cargo clippy -- -D warnings  # zero warnings, zero lints
cargo fmt --check         # no formatting differences
cargo test                # all tests pass
```

No `#[allow(...)]` suppressions without an inline comment explaining why the lint is a false positive in that specific case.

---

## Ownership and memory rules

```rust
// Prefer &str over String in parameters
fn greet(name: &str) { ... }
 
// Prefer &Path over &PathBuf in parameters
fn read_file(path: &Path) { ... }
 
// Use impl Into<T> for ergonomic public APIs
fn new(name: impl Into<String>) -> Self { ... }
 
// Use with_capacity when size is known
let mut v: Vec<String> = Vec::with_capacity(mappings.len());
 
// Use Path::join — never string concatenation for paths
let dest = home.join(".config/dotm/config.toml");
```
 
- Never `.clone()` inside a loop unless the clone is genuinely required each iteration
- Use `std::mem::take()` to move out of mutable references instead of cloning

---

## Safety rules

- `unsafe` blocks are forbidden unless there is no safe alternative
- Every `unsafe` block must have a `// SAFETY:` comment that states the invariant being upheld
- No raw pointer arithmetic without a preceding design discussion
- Symlink operations must validate that the resolved destination is within the expected directory tree (prevent path traversal)
- User-supplied paths must be sanitized before being passed to `std::fs` functions
- Structs that contain sensitive data (tokens, passwords) must NOT derive `Debug` — implement it manually and redact the sensitive fields

```rust
// WRONG — token leaks into logs
#[derive(Debug)]
struct GithubConfig {
    token: String,
}
 
// CORRECT
impl fmt::Debug for GithubConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GithubConfig")
            .field("token", &"[REDACTED]")
            .finish()
    }
}
```

---

## Testing rules
 
```rust
// Unit tests at the bottom of each file
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
 
    #[test]
    fn test_expand_tilde_resolves_home() {
        let result = expand_tilde("~/.zshrc").unwrap();
        assert!(result.to_string_lossy().contains(".zshrc"));
        assert!(!result.to_string_lossy().starts_with('~'));
    }
 
    #[test]
    fn test_expand_tilde_returns_error_on_missing_home() {
        // Test error path, not just happy path
    }
}
```
 

- Every public function needs at least one unit test covering the happy path
- Every public function that returns `Result` needs at least one test covering the error path
- Use `tempfile::TempDir` for any test that touches the filesystem — never use hardcoded paths
- Integration tests live in `tests/` and invoke the binary via `assert_cmd::Command::cargo_bin("dotm")`
- Test names use snake_case and describe the scenario: `test_install_creates_symlink_for_local_source`
- No `#[ignore]` on tests without a tracking comment explaining when they will be re-enabled

---

## Git and versioning

- Commit messages follow Conventional Commits: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`
- Every feature branch targets `main`
- Version in `Cargo.toml` follows SemVer: breaking change = major bump, new feature = minor bump, bug fix = patch bump
- Tag format for releases: `v0.1.0`

---

## Output and UX conventions

All terminal output follows this format:

```
  ✓  green   — success / already correct
  →  cyan    — informational / in progress
  ~  yellow  — warning / modified / dry-run
  ✗  red     — error / missing / failed
  !  yellow  — conflict / attention needed
```

- Dry-run mode must prefix every simulated action with `[dry-run]`
- Long operations (git clone, git pull) must show a spinner via `indicatif`
- Error messages must be printed to `stderr`, never `stdout`
- Exit code 0 on success, 1 on any error

---

## What all agents must respect

1. **Planner** defines the architecture — the Developer does not deviate from it without flagging the change
2. **Developer** produces code that passes all quality gates before handoff to the Reviewer
3. **Reviewer** blocks any code with CRITICAL or HIGH issues — no exceptions
4. All three agents share this skill file as the single source of truth for project standards
5. When this skill conflicts with an agent's own instructions, **this skill takes precedence**