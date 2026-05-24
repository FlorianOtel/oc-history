# oc-history — project-specific instructions for Claude Code

This file augments the global `~/.claude/CLAUDE.md`. Anything here is specific to this repository.

## What this project is

`oc-history` is a Rust TUI for browsing and managing **opencode** session history. It is a downstream **consumer / viewer / manager** — it never creates sessions. opencode sessions are produced by upstream tools like `octmux` (see `~/Gin-AI/projects/octmux/`) talking to opencode's headless HTTP endpoint.

The repo is forked from `raine/claude-history` (a similar TUI for Claude Code's JSONL sessions). Pre-fork history is preserved as an appendix at the bottom of `docs/Changelog.md`.

## Architecture (locked in v0)

**Pure HTTP** against opencode's headless endpoint. No direct SQLite access against `~/.local/share/opencode/opencode.db` anywhere in this codebase. All session reads go via `GET /session` / `GET /session/{id}/message`; all mutations via `DELETE /session/{id}` (and, in later stages, `PATCH /session/{id}`).

Default endpoint: `http://127.0.0.1:4096` (octmux's default). Override via `--endpoint` CLI flag or `OPENCODE_BASE_URL` env var. Hard error on unreachable; no offline mode.

Open architectural question (cross-session fuzzy search under pure HTTP) is tracked in `docs/Implementation-plan.md` → `Open Questions`. Don't accidentally introduce a SQLite read path while solving it without revisiting that decision.

## Workflow rules

### Release-build verification after approved review

**After every approved review of changes** — whether that's a `/brain` Reviewer **PASS**, a `/duo` ack, or a direct user approval of a diff — run `cargo build --release` as a final gate **before** declaring the work complete or committing.

The debug build (`cargo build`) skips overflow checks, uses different panic ABI, and can miss issues that only surface under `--release`. Catch them at this gate, not in the user's terminal.

If `cargo build --release` errors, treat the work as not-done: surface the error, dispatch Actor (in `/brain`) or fix directly (outside the pipeline), and re-run. Warnings are acceptable; errors are not.

### Documentation discipline

- The implementation staging document is `docs/Implementation-plan.md`. Stages are added as the project grows; each stage's status marker (`🟡 not started` / `🟢 in progress` / `✓ shipped — see Changelog YYYY-MM-DD--HH-MM`) is flipped as work moves through it.
- The change log is `docs/Changelog.md` (mixed case — **not** `CHANGELOG.md`). Discipline is documented at the top of the file itself; follow it.
- Both files use the project's standard frontmatter (title / created_at / created_by / updated_by / updated_at / context). Refresh `updated_by` and `updated_at` on every edit.

### Code conventions

- Rust **edition 2021** (not 2024) — the toolchain on this machine doesn't reliably handle let-chain syntax under 2024. If you find yourself wanting to write `if let Some(x) = y && cond`, rewrite to nested `if let` or use a tuple match.
- v0 retains a fair amount of dead code from the claude-history fork (`src/claude.rs`, `src/display.rs`, `src/history/{loader,parser,path,cache,global_log}.rs`, parts of `src/tui/{search,viewer,export}.rs`) as `#[allow(dead_code)]` / stub shims. Full cleanup is deferred to **v1**. Don't delete these in unrelated work; v1 owns the sweep.
- Functions referenced by dead-code paths (`find_jsonl_by_uuid`, `process_conversation_file` in `src/history/mod.rs`) are stubs that return errors — runtime-unreachable in v0. If they become reachable, that's a bug.

### Running the tool

Manual end-to-end verification needs an opencode server running on `127.0.0.1:4096`:

```bash
~/.opencode/bin/opencode serve --hostname 127.0.0.1 --port 4096   # or attach to octmux's instance
cargo run --release
```

`octmux`'s default-mode behaviour is to attach to a pre-existing opencode instance on 4096 (systemd service `scripts/opencode-server.service` in octmux). If octmux is in use, that same instance backs `oc-history`.

## Pipeline notes

- `/brain` pipeline artefacts live under `.claude/orchestra/sessions/<timestamp>/` and are operational (not committed). They include `RESEARCH.md`, `PLAN.md`, `TASKS.json`, `review-comments.md`, `telemetry.json`.
- The orchestra in-pipeline guard from the global CLAUDE.md applies here: while `.brain-inflight` or `.duo-inflight` exists, code edits must go through Actor (`subagent_type: actor`), not direct Edit/Write/Bash.
