---
title: "Stage v0.5 — per-project session filter (TAB toggle): effort evaluation"
created_at: 2026-05-24--18-48
created_by: Claude Code (Claude Opus 4.7)
context: >
  Scoping document produced after v0 shipped (commit 04cfca1, 2026-05-24).
  Florian asked for an effort evaluation — not an implementation — of reviving
  claude-history's "TAB filters by project" behaviour, ported to opencode
  semantics. This document captures the investigation results and proposed
  approach so a future session can pick up v0.5 cleanly without re-discovering
  the scaffolding that v0 retained under #[allow(dead_code)].
---

# Stage v0.5 — per-project session filter (TAB toggle)

## Context

v0 of `oc-history` shipped on 2026-05-24 (commit `04cfca1`) — bare list TUI + safe delete over pure HTTP against opencode's headless endpoint. The next small feature on the list is reviving claude-history's "TAB filters by project" behaviour, ported to opencode semantics:

- Pressing TAB on a highlighted session shows only the sessions whose `directory` (opencode's project) matches the highlighted one's.
- Pressing TAB again restores the full list.
- This is still session-listing — same six columns, same data model, no rendering changes — purely an in-memory filtered view.

This document is an **effort evaluation**, not an implementation plan to be executed in this turn. It scopes the work so a future session can decide whether to slot v0.5 in before v1 (text viewer).

## Effort verdict: small (~30–60 min Actor time, ~1.5 h end-to-end via the `/brain` pipeline)

The claude-history TAB-filter scaffolding is **already compile-active in v0** — the prior `/brain` Actor retained it under the deferred-cleanup policy documented in the project `CLAUDE.md`. The user-facing surface is essentially drawn; the work is three small wiring fixes plus a semantic shift.

## What's already wired (compile-active in v0)

- `src/tui/app.rs:1721, 1915` — `KeyCode::Tab → toggle_workspace_filter()` in both list-mode and search-mode input handlers.
- `src/tui/app.rs:293–295` — App fields `workspace_filter: bool` and `current_project_dir_name: Option<String>`, both initialised in `App::new()`.
- `src/tui/app.rs:563–603` — `update_filter()` applies the workspace filter to the indices list.
- `src/tui/app.rs:785, 834–849` — `filtered()`, `workspace_filter()`, `has_project_context()`, `toggle_workspace_filter()` getters/setters.
- `src/tui/ui.rs:233–246` — status-bar `Tab·Prj` / `Tab·All` indicator (gated on `has_project_context()`).
- `src/tui/ui.rs:1116` — help-legend entry `"Tab → Toggle scope (All/Project)"`.
- `src/tui/ui.rs:1210–1218` — rendering iterates through `app.filtered()` (Vec<usize> of indices), so a filtered list paints automatically.
- Background search worker in `src/tui/app.rs:164–242` threads `workspace_filter` + `project_dir_name` and applies them in its inner loop.

The data is also already in place: `Conversation.project: String` is populated from opencode's `session.directory` at `src/opencode/loader.rs:81`.

## What's broken / missing in v0

1. **`current_project_dir_name` is never set.** In claude-history this came from detecting the user's launch CWD and encoding it into the mangled `-mnt-nfs-...` projects-dir name. v0 has no such init path, so `has_project_context()` returns `false` and `toggle_workspace_filter()` no-ops.

2. **The semantics need to change** — and they get simpler. The spec: TAB pivots on the **highlighted conversation's project**, not on a CWD captured at launch. TAB grabs `conversations[selected].project` and pins it as the active filter target. Pressing TAB on a different highlighted session later re-pins.

3. **Both filter-comparison sites use claude-history's mangled-path logic.** `update_filter()` at `src/tui/app.rs:580–595` and the search worker at `src/tui/app.rs:230–242` both dispatch through `conv.path.parent().file_name()` and `crate::history::path::is_same_project()`. In v0, `conv.path` is a stub `PathBuf::new()`, so the predicate is always false. opencode's `directory` is a plain absolute-path string — exact-string equality is the right comparator for v0.5; worktree coalescing is explicitly out of scope.

## Recommended approach

1. **`toggle_workspace_filter()` pivots on the highlighted conversation.** On enable, set `current_project_dir_name = Some(conversations[filtered[selected]].project.clone())`, flip `workspace_filter = true`, call `update_filter()`. On disable, flip `workspace_filter = false` and re-`update_filter()`. The next TAB-on re-pins fresh from whatever is highlighted at that moment.

2. **Simplify both filter sites to exact-string match on `conv.project`.**
   - `src/tui/app.rs:580–595` (`update_filter` workspace branch): replace the `conv.path`-based logic with `conversations[idx].project == *pinned_project`.
   - `src/tui/app.rs:230–242` (search worker workspace branch): same replacement.
   - Leave `crate::history::path::is_same_project` untouched (still dead-code; v1's cleanup pass owns deletion; v6 may revisit for worktree coalescing).

3. **`has_project_context()` returns true when at least one conversation is loaded with a non-empty `project`.** Cheapest: `!self.conversations.is_empty()`. This makes the status-bar `Tab·All` / `Tab·Prj` label appear as soon as the loader's first batch arrives.

4. **No changes needed** to `Conversation`, `opencode::*`, the renderer, the data flow, or the search query / UUID paths.

## Files to modify

- `src/tui/app.rs` — `toggle_workspace_filter`, `update_filter` (workspace branch), `has_project_context`, search-worker inner loop body. Net ~40–60 lines touched.
- `docs/Implementation-plan.md` — add v0.5 stage between v0 and v1 with Assumptions / Goal / In scope / Out of scope / Deliverables / Tests / Handover; flip marker `🟡` → `🟢` → `✓` as work progresses.
- `docs/Changelog.md` — add v0.5 entry once shipped.

No other files need changes.

## Verification

Manual end-to-end against a live opencode endpoint (`127.0.0.1:4096`):

1. `cargo run --release` with sessions across at least two different directories.
2. Highlight a session in dir A → press TAB → list reduces to dir-A sessions; status bar shows `Tab·Prj`.
3. Press TAB again → full list restored; status bar shows `Tab·All`.
4. Highlight a session in dir B → press TAB → list reduces to dir-B sessions (re-pinned).
5. Type a search query, then toggle TAB → filter composes with search.
6. Delete a session while filter is active → list refreshes and filter still holds.

Plus: `cargo build --release` clean (final gate per project `CLAUDE.md`).

## Risks / open items

- **Worktree coalescing** (treating `/foo/repo` and `/foo/repo-wt` as the same project) is **explicitly deferred** — out of scope for v0.5. Document under v0.5's "Out of scope" in `Implementation-plan.md`; possibly add a new `Open Questions` entry if it becomes a recurring ask.
- **Empty `conv.project`**: opencode's `directory` is non-optional in the SDK schema (`src/opencode/models.rs:8`), but any session with an empty string would coalesce into one "empty" group under exact-match. Acceptable edge case; document under Handover notes.
- **Search worker concurrency**: the worker pulls `workspace_filter` + `project_dir_name` from the `SearchCommand::Search` payload at dispatch time, so changes to the App fields between dispatch and worker-receive are race-free by construction. No new locking needed.

## Effort summary

| Phase | Estimate |
|---|---|
| Actor code changes (~40–60 lines, 1 file) | 30–45 min |
| Reviewer (small diff, clear scope) | 15 min |
| `cargo build --release` gate + manual TUI verification | 15 min |
| Docs (Implementation-plan v0.5 entry + Changelog entry) | 10 min |
| `/brain` pipeline overhead (Phase 0/1, persistence, telemetry) | 15 min |
| **Total wall-clock** | **~1.5 hours** |

## Recommendation

Worth slotting in as v0.5 before v1, on two grounds:

1. The scaffolding is already present and would otherwise rot as v1's cleanup pass deletes related dead code — fixing it now keeps the deletions simpler.
2. It's immediately useful for any user with sessions across multiple project directories (which is the default opencode + octmux workflow).

If v1 is the higher priority, defer cleanly: the dead-code retention from v0 is intentional and v0.5 can land any time before or after v1 without architectural conflict.

## Handover for the next session

When picking this up:

1. Re-confirm the four file:line references above still exist (line numbers may drift if any v1 refactor lands first).
2. Decide whether to run as a `/brain` (research → plan → actor → review) or a `/duo` (small-and-bounded enough that the review loop may be overkill). The diff is well under 100 lines — `/duo` is defensible.
3. Author should pin the matching v0.5 stage entry in `docs/Implementation-plan.md` **before** Actor begins, so the stage-marker progression is visible alongside the code change.
4. Don't delete `src/history/path::is_same_project` — v1's cleanup pass owns that.
