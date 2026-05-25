---
title: "oc-history — Implementation Plan"
created_at: 2026-05-24--11-16
created_by: Claude Code (Claude Sonnet 4.6)
updated_by: Claude Code (Claude Haiku 4.5)
updated_at: 2026-05-25--00-00
context: >
  Implementation staging plan for the oc-history port. The repository is a verbatim
  Rust fork of claude-history (a TUI session browser for Claude Code). The goal is to
  replace the Claude Code JSONL / filesystem data layer with pure HTTP calls against
  the opencode headless endpoint and ship a TUI that reaches feature parity with the
  original tool. Each stage is sized to fit in a single Claude Code session with
  minimal context carry-over. This document is the authoritative staging record;
  docs/Changelog.md carries the per-stage completion log.
---

# oc-history — Implementation Plan

This document tracks the multi-stage port of `oc-history` from its `claude-history`
origin to opencode. Each stage carries:

- **Assumptions** — repo state that must be true entering this stage.
- **Goal** — what the stage achieves, in one paragraph.
- **In scope / Out of scope** — explicit fencing.
- **Deliverables** — files created or modified.
- **Tests** — manual / automated verification.
- **Handover notes for the next stage** — assumptions the next stage may rely on.

Each stage header carries one of:

- `Status: 🟡 not started`
- `Status: 🟢 in progress`
- `Status: ✓ shipped — see Changelog YYYY-MM-DD--HH-MM`

Stages correspond 1:1 with entries in `docs/Changelog.md`. When a stage ships,
update the marker here and append the Changelog entry there.

---

## Open Questions

Architectural / scope questions to resolve later. Each entry is timestamped and
describes the issue, the current best-known mitigation, and where the resolution
will land.

### 2026-05-24 — Cross-session fuzzy search under pure-HTTP

**Issue.** The opencode HTTP API exposes no native search primitive. `claude-history`
ships a `/`-triggered global search that scans every session's content; reconstructing
that under pure HTTP forces us to materialise the search corpus client-side. Possible
strategies:

1. **Eager fetch on every search session** — fire `GET /session/{id}/message`
   concurrently for every session at search time. ~hundreds of ms to seconds at
   >100 sessions on localhost; painful over the wire.
2. **Persisted local index** — port `claude-history`'s existing bincode cache
   to be HTTP-backed; invalidate per session by comparing `session.time.updated`
   against the indexed value (one `GET /session` call). Faster cold-start after
   first run; requires cache schema + invalidation logic.
3. **SSE-fed live index** — subscribe globally to `/sse/global/event` for the
   tool's lifetime; apply `message.part.delta` / `message.part.updated` events
   to the in-memory index incrementally. Persist on exit. Strongest correctness,
   most plumbing.
4. **SQLite escape hatch (last resort)** — read-only SQLite fallback against
   `~/.local/share/opencode/opencode.db` used **only** for the index build path.
   Schema-coupling cost paid once, for one feature, not the whole tool.

**Pure-HTTP drawbacks that motivate this question.**

- Hard dependency on the opencode HTTP endpoint being reachable; no offline mode.
- First-paint latency on the list view is bounded by N parallel `GET /session/{id}/message` calls (for turn count). Acceptable on localhost; painful over the wire.
- SDK schema-drift risk: opencode's TS SDK is the de-facto HTTP contract; Rust models must be hand-rolled and tracked as opencode evolves.
- Live-follow via SSE adds connection-lifecycle complexity (reconnect, backoff, dedup). Accepted as part of v4 scope.

**Provisional mitigation (as of 2026-05-24).** v0..v5 ship without any cross-session
search. v6 is provisionally scoped to ship option (2) — persisted bincode index
with `time.updated`-based invalidation. Options (3) and (4) remain as escape
hatches if (2) proves insufficient.

**Decision needed by.** Start of v6. Until then v0..v5 can proceed without
committing.

---

## Stage v0 — Bare list + safe delete

Status: ✓ shipped — see Changelog 2026-05-24--11-45

### Assumptions

- Repository is a verbatim fork of upstream `claude-history` at
  `/mnt/nfs/Florian/Gin-AI/projects/oc-history`. `cargo build` currently succeeds.
- An opencode binary exists at `~/.opencode/bin/opencode` and can be started in
  headless mode listening on `127.0.0.1:4096`.
- `docs/Changelog.md` carries the per-stage log discipline; `docs/Implementation-plan.md`
  is empty before v0 ships (this file is its first content).

### Goal

Ship a working `oc-history` binary that connects to the opencode HTTP endpoint,
lists all sessions with six columns (title, turns, project, started, cost,
tokens), and allows safe deletion of sessions via a confirmation dialog. All
session-content rendering is stubbed.

### In scope

- `src/opencode/` HTTP client module: `Session`, `MessageEnvelope` models;
  `list_sessions`, `list_messages`, `delete_session`, `probe_health`.
- List TUI with six columns: title · turns · project · started · cost · tokens.
  Sort: `time.updated` descending.
- Safe delete: existing `ConfirmDelete` dialog → `DELETE /session/{id}` →
  200/404/refused surfacing.
- Binary identity: `Cargo.toml` package + binary name + version + help → `oc-history`.
- CLI: `--endpoint <URL>` + `OPENCODE_BASE_URL` env var; default
  `http://127.0.0.1:4096`.
- Startup health probe: `GET /health` with `GET /session` fallback; hard error
  with explicit URL + override hint on failure.
- Enter-key stub: status message "Session viewer: deferred to v1".
- `docs/Implementation-plan.md` (this file) created.
- `docs/Changelog.md` v0 entry appended.

### Out of scope

Everything in v1+ (session viewer, tool/thinking display, search, SSE, rename,
scope toggle).

### Deliverables

Created:

- `src/opencode/mod.rs`, `src/opencode/models.rs`, `src/opencode/client.rs`,
  `src/opencode/loader.rs`
- `docs/Implementation-plan.md` (this file)

Modified:

- `Cargo.toml` (package rename, HTTP client dep)
- `src/cli.rs` (rename, add `--endpoint`, drop claude-only flags)
- `src/main.rs` (rewire `run()` to opencode loader + client)
- `src/history/mod.rs` (gut to compile-stub re-exports)
- `src/tui/app.rs` (rewire delete; stub `enter_view_mode`)
- `src/tui/ui.rs` (rewire list rendering to six opencode columns)
- `src/error.rs` (add `EndpointUnreachable` variant)
- `docs/Changelog.md` (v0 entry + frontmatter `updated_*`)

### Tests

1. `cargo build` succeeds with no errors.
2. `oc-history --endpoint http://127.0.0.1:9999` → exits 1 with
   "cannot reach opencode" message naming the URL + override hint.
3. With opencode running on 4096: `oc-history` → TUI opens, sessions listed
   with six columns visible, newest first.
4. Press `d` + `y` on a throwaway session → session removed; re-running confirms it stays gone.
5. Press `Enter` on any session → status bar: "Session viewer: deferred to v1 — press Esc to return".
6. `oc-history --version` → "oc-history X.Y.Z".

### Handover notes for v1

- `src/opencode/client.rs::list_messages` already returns parts as
  `serde_json::Value`; v1 extends it to parse `Part` variants.
- `src/tui/app.rs::enter_view_mode` is the stub to replace with real rendering.
- `src/tui/viewer.rs` and `src/display.rs` contain old claude-specific rendering;
  to be replaced or gutted in v1.
- `src/history/{loader,parser,path,cache,global_log}.rs`, `src/claude.rs`,
  `src/display.rs` are dead code after v0 — safe to delete in v1 cleanup.

---

## Stage v0.5 — per-project session listing (TAB title-scoped filter)

Status: ✓ shipped — see Changelog 2026-05-24--20-33

### Assumptions

- v0 shipped: bare list TUI, safe delete, pure HTTP, commit `04cfca1`.
- `src/tui/app.rs` workspace-filter scaffolding is compile-active (fields, key handlers, UI labels all wired; only filter predicate and init path broken).

### Goal

Wire the already-scaffolded TAB workspace-filter so that pressing TAB on a highlighted
session narrows the list to sessions with the same title; pressing TAB again restores the
full list. Also fix the `GET /project` integration so `conv.project` reflects the actual
project worktree path, and fix the broken search index (was `String::new()` for all sessions).

**Implementation note:** The original spec assumed `session.directory` would vary per
session. In practice this deployment uses opencode global mode — all sessions have
`projectID='global'` with identical `directory` and `project.worktree='/'`. The filter
therefore pivots to exact `conv.title` match, which provides meaningful grouping in practice.

### In scope

- `Project` model + `list_projects()` HTTP call; loader fetches projects at startup and
  uses `project.worktree` for `conv.project` + `project_name`.
- `search_text_lower` populated from `title + project_short` (v0 stub was `String::new()`).
- `toggle_workspace_filter()`: pivot on highlighted session's `.title`.
- `update_filter()` workspace branch: exact match on `conv.title`.
- Search worker workspace branch: same predicate.
- `has_project_context()`: return `!self.conversations.is_empty()`.
- `current_project_name()` accessor; UI prompt renders pinned title when filter active.

### Out of scope

- Worktree coalescing — deferred.
- `crate::history::path::is_same_project` — not modified; v1 cleanup owns deletion.
- Session viewer (v1).

### Deliverables

Modified:
- `src/opencode/models.rs` (`Project` struct)
- `src/opencode/client.rs` (`list_projects()`)
- `src/opencode/mod.rs` (re-export `Project`)
- `src/opencode/loader.rs` (project map, `search_text_lower`)
- `src/tui/app.rs` (filter logic, `current_project_name`)
- `src/tui/ui.rs` (search prompt with pinned title)
- `docs/Implementation-plan.md` (this file)
- `docs/Changelog.md` (v0.5 entry)

New:
- `docs/Stage-v05.md` (planning doc, committed with this stage)

### Tests

1. `cargo build --release` clean.
2. With opencode running on 4096:
   a. Tab indicator shows `Tab·All` as soon as sessions load.
   b. Typing a title keyword → matching sessions appear (search fix).
   c. Highlight a session → TAB → list narrows to sessions with same title; indicator `Tab·Prj`; prompt shows title.
   d. TAB again → full list; indicator `Tab·All`.
   e. Type search query with filter active → intersection of search + filter.

### Handover notes for v1

- `current_project_dir_name` stores the pinned title (repurposed field). Re-pin on next
  TAB-on is correct behaviour and already implemented.
- The `GET /project` integration is correct and future-proof: if a future deployment
  uses per-session projects, `conv.project` will reflect the real worktree path and
  the title-based filter could be replaced with a project-based one.
- Worktree coalescing is explicitly deferred — see Open Questions in this file.

---

## Stage v1 — Session content viewer (text-only)

Status: ✓ shipped — see Changelog 2026-05-24--23-11

### Assumptions

- v0 has shipped; `oc-history` binary exists with working list + delete.
- `src/opencode/Client::list_messages(id)` returns
  `Vec<MessageEnvelope { info, parts: Vec<serde_json::Value> }>`.
- `src/tui/app.rs::enter_view_mode` is a stub that sets a "deferred" status
  message.

### Goal

Pressing Enter on a session opens a scrollable viewer showing the conversation
as plain text — user prompts and assistant text responses only, no tool calls,
no reasoning, no timing. Existing nav keys (j/k, Ctrl-D/U, gg/G, Page-Up/Down)
work via the existing `ViewState`.

### In scope

- Extend `opencode::models` with typed `Part` enum:
  `Part::Text { text: String }`, `Part::Unknown { type_: String }` (forward-compat).
- New view model: `OcSessionView { messages: Vec<MessageView> }` where
  `MessageView { role, time, text_parts: Vec<String> }`.
- Replace `enter_view_mode` stub with real implementation: fetch messages →
  flatten text parts → build `Vec<RenderedLine>` → transition to `AppMode::View`.
- Role headers ("User" / "Assistant") with timestamp from `info.time.created`.
- Update `src/tui/viewer.rs` to accept `OcSessionView` instead of a file path.
- **Cleanup:** delete `src/claude.rs`, `src/display.rs`,
  `src/history/{loader,parser,path,cache,global_log}.rs`. Strip dangling imports.

### Out of scope

- Tool / reasoning / timing display.
- Within-viewer search.
- Message-level navigation (J/K/[/]).

### Deliverables

Modified:

- `src/opencode/models.rs` (add `Part`, `Part::Unknown`)
- `src/opencode/client.rs` (`fetch_session_content(id) -> OcSessionView`)
- `src/tui/viewer.rs` (rewrite for opencode data)
- `src/tui/app.rs` (`enter_view_mode` replaced)
- `docs/Changelog.md` (v1 entry); `docs/Implementation-plan.md` (marker flip).

Deleted: `src/claude.rs`, `src/display.rs`, `src/history/{loader,parser,path,cache,global_log}.rs`.

### Tests

1. Press Enter on a session → viewer opens with user/assistant turns visible.
2. Scroll with j/k and Page-Down.
3. Press Esc → returns to list.
4. Empty session → viewer shows "No messages" placeholder.
5. `cargo build` succeeds after dead-code deletion.

### Handover notes for v2

- Add `show_tools: bool`, `show_thinking: bool`, `show_timing: bool` to
  `ViewState` now (even if unused) so v2 only changes rendering logic.
- `Part::Unknown` is the catch-all for variants v2 will handle.

---

## Stage v2 — Tool calls, thinking blocks, timing markers

Status: ✓ shipped — see Changelog 2026-05-25--00-00

### Assumptions

- v1 shipped; viewer renders text-only conversations.
- `ViewState` carries `show_tools`, `show_thinking`, `show_timing` flags.
- `Part::Unknown` is the catch-all in `opencode::models`.

### Goal

Viewer shows tool calls (input + output), reasoning blocks (toggle `T`), and
per-message timing markers (toggle `i`). `t` cycles tool display
Hidden → Truncated → Full.

### In scope

- Extend `Part` variants:
  - `Part::Tool { state: ToolState }` where
    `ToolState { status: "pending"|"running"|"completed"|"error", input, output, time }`.
  - `Part::Reasoning { text }`.
  - `Part::StepStart`, `Part::StepFinish` (timing markers).
- Render each variant honoring the three flags.
- Key bindings (`t`, `T`, `i`) already wired in `app.rs` — connect to renderer.
- Adapt or replace `src/tool_format.rs` for opencode tool input/output shapes.

### Out of scope

- Search; message navigation.
- `Agent`, `Subtask`, `Retry`, `Compaction` part rendering (placeholder text only).

### Deliverables

Modified:

- `src/opencode/models.rs` (extend `Part`)
- `src/tui/viewer.rs` (render new part types)
- `src/tool_format.rs` (adapt for opencode)
- `docs/Changelog.md` (v2 entry); `docs/Implementation-plan.md` (marker flip).

### Tests

1. Session with tool calls: press `t` → cycle Hidden/Truncated/Full.
2. Session with reasoning: press `T` → toggle.
3. Press `i` → timing markers visible.
4. `cargo build` succeeds.

### Handover notes for v3

- `ViewState::message_ranges`, `focused_message`, `message_nav_active` are
  already present; v3 activates them.
- v2 renderer must populate `rendered_lines` so v3 search can index over it.

---

## Stage v3 — Within-viewer navigation + search

Status: 🟡 not started

### Assumptions

- v2 shipped; viewer renders tool calls, reasoning, timing.
- `rendered_lines` is populated and reflects current toggles.
- `ViewState::message_ranges` is populated by the renderer.

### Goal

`J`/`K` jump to next/prev message; `/` (forward) and `?` (backward) search;
`n`/`N` cycle matches.

### In scope

- `J`/`K`/`[`/`]` keys: wire to `message_ranges` (logic already in `handle_view_key`).
- `/` and `?` open `ViewSearchMode::Typing`; search logic in `app.rs` already
  complete — it searches `rendered_lines`.
- `n`/`N` cycle matches.

### Out of scope

- Cross-session fuzzy search.
- SSE live follow.

### Deliverables

Modified:

- `src/tui/viewer.rs` (ensure `message_ranges` populated)
- `src/tui/app.rs` (any small wiring fixes)
- `docs/Changelog.md` (v3 entry); `docs/Implementation-plan.md` (marker flip).

### Tests

1. Press `J` → jumps to next message start.
2. Press `/foo` Enter → highlights occurrences.
3. `n`/`N` cycle matches.
4. Esc clears search.

### Handover notes for v4

- SSE global event stream (`GET /sse/global/event`) is the foundation for v4
  live-follow. v3 doesn't need it; v4 opens it on viewer entry.

---

## Stage v4 — Live follow via SSE

Status: 🟡 not started

### Assumptions

- v3 shipped; viewer is static-rendered with navigation and search.
- `src/opencode/client.rs` exposes basic HTTP operations; SSE is new in v4.

### Goal

When a session is open and its status is `busy`, the viewer follows new messages
and parts in real time via SSE. New content appends automatically; on
`session.idle` the viewer shows a "Completed" indicator.

### In scope

- `src/opencode/sse.rs`: subscribe to `GET /sse/global/event`; filter by
  session ID; emit normalised events to a channel.
- Apply `message.part.delta` / `message.part.updated` to in-memory parts.
- Reconnect with exponential backoff on connection drop.
- SSE reader thread → main TUI loop via `mpsc`, polled in the event loop.
- Unsubscribe on viewer exit.

### Out of scope

- Global list-level SSE (new sessions auto-appearing in list).
- Multi-session parallel follow.

### Deliverables

Created: `src/opencode/sse.rs`

Modified:

- `src/tui/app.rs` (SSE channel integrated into event loop)
- `docs/Changelog.md` (v4 entry); `docs/Implementation-plan.md` (marker flip).

### Tests

1. Open a session in oc-history; from another terminal, send a prompt to opencode.
2. New text streams into the viewer.
3. On `session.idle` → "Completed" indicator.
4. Kill opencode → viewer shows disconnect; restart → reconnect.

### Handover notes for v5

- v5's workspace scope toggle triggers a new `list_sessions` call with
  `?directory=`; the SSE subscription (global) is unaffected.

---

## Stage v5 — Workspace scope toggle + in-TUI session rename

Status: 🟡 not started

### Assumptions

- v0..v4 shipped.
- `Conversation.project` carries `session.directory`.

### Goal

`Tab` toggles between "All sessions" and "This directory" (`session.directory == $PWD`).
`r` opens an inline rename prompt; on submit, sends `PATCH /session/{id}` with
`{ title: ... }`.

### In scope

- `Tab` toggles `workspace_filter`; filter uses `std::env::current_dir()` matched
  against `conv.project`.
- `AppMode::Rename` (or `DialogMode::RenameSession`) with inline text input.
  Enter → `client.rename_session(id, new_title)` → update `conv.title` in memory.
  Esc → cancel.
- `src/opencode/client.rs`: add `rename_session(id, title)`.

### Out of scope

- Cross-session fuzzy search.
- Multi-select / archive / export.

### Deliverables

Modified:

- `src/opencode/client.rs` (`rename_session`)
- `src/tui/app.rs` (`AppMode::Rename`)
- `src/tui/ui.rs` (rename prompt rendering)
- `docs/Changelog.md` (v5 entry); `docs/Implementation-plan.md` (marker flip).

### Tests

1. Tab → list narrows to sessions under `$PWD`.
2. Tab again → all sessions restored.
3. `r` → prompt; type new name; Enter → title updates in list.
4. Esc → prompt cancelled, old name retained.

### Handover notes for v6

- See **Open Questions → Cross-session fuzzy search under pure-HTTP**. The
  decision between options (2)/(3)/(4) must be made at the start of v6.

---

## Stage v6 — Cross-session fuzzy search with local index

Status: 🟡 not started

**Pending decision.** See **Open Questions → Cross-session fuzzy search under
pure-HTTP**. The scope below assumes option (2): persisted bincode index with
`time.updated`-based invalidation.

### Assumptions

- v0..v5 shipped.
- Open Questions entry on cross-session search has been resolved to option (2).
- `bincode` and `indicatif` are in `Cargo.toml` (claude-history deps, kept).

### Goal

`/`-triggered search across all session titles and message text, backed by a
persistent bincode index at `~/.cache/oc-history/index.bincode`. First run builds
the index (progress bar); subsequent runs load it and refetch only sessions whose
`time.updated` advanced.

### In scope

- Index struct: `HashMap<SessionId, IndexedSession>` (normalised text + metadata).
- Staleness check via `time.updated` from `GET /session` vs indexed value.
- Background index build on first run with `indicatif` progress.
- Port `tui/search.rs` TF-IDF scorer to `IndexedSession` shape.
- Index header carries SDK version hash; rebuild on version change.

### Out of scope

- SSE-fed incremental index updates (later).
- Shared cross-process index (file lock for writes is future work).

### Deliverables

Created: `src/opencode/index.rs`

Modified:

- `src/tui/search.rs` (work over `IndexedSession`)
- `src/tui/app.rs` (search dispatch wired to index)
- `docs/Changelog.md` (v6 entry); `docs/Implementation-plan.md` (marker flip).

### Tests

1. First run: progress bar "Indexing N sessions".
2. Second run: starts instantly.
3. Search for a known word → that session appears in results.
4. Create a new opencode session; re-run oc-history → new session appears in search.

### Handover notes for post-v6

- Export / copy / resume / fork actions (stubbed since v0) candidate for v7.
- Multi-select delete is a natural follow-on to v0's delete flow.

---

## Marker convention reference

Each stage header carries one of:

```
Status: 🟡 not started
Status: 🟢 in progress
Status: ✓ shipped — see Changelog YYYY-MM-DD--HH-MM
```

Update the marker here and add the Changelog entry when the stage ships.
