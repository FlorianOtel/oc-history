---
title: "oc-history — Implementation Plan"
created_at: 2026-05-24--11-16
created_by: Claude Code (Claude Sonnet 4.6)
updated_by: Claude Code (Claude Haiku 4.5)
updated_at: 2026-05-27--15-15
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

**Provisional mitigation (as of 2026-05-25).** v0..v5 ship without any cross-session
search. v6 is provisionally scoped to ship option (2) — persisted bincode index
with `time.updated`-based invalidation. Options (3) and (4) remain as escape
hatches if (2) proves insufficient.

**Decision needed by.** Start of v6. Until then v0..v5 can proceed without
committing.

---

### 2026-05-25 — SSE live streaming in viewer not working

**Issue.** The v4 SSE live-follow feature was partially implemented but real-time
streaming into the viewer pane does not work in practice. The workaround is to exit
the viewer and re-enter it — this re-fetches session content via `GET /session/{id}/message`
and displays the latest exchange correctly, but not in real time.

**What was attempted.**

1. Created `src/opencode/sse.rs`: a background thread subscribes to
   `GET {base_url}/global/event` (opencode's global SSE stream), reads lines via
   `BufReader`, parses `data: {json}` lines, and sends normalised `SseEvent`
   variants (`ContentChanged`, `SessionIdle`, `Reconnecting`, `Failed`) to the main
   loop via `mpsc::channel`.
2. The main loop polls the channel every 100 ms; on `ContentChanged` it calls
   `client.fetch_session_content()` and re-renders the viewer.
3. Several bugs were fixed iteratively:
   - **Wrong endpoint**: originally `GET /sse/global/event`; corrected to
     `GET /global/event` (matching the opencode SDK's `client.global.event()` call).
   - **Wrong sessionID path**: original code looked up `event["sessionID"]`
     (top-level, always absent); corrected to per-type paths
     (`event["properties"]["sessionID"]`, `event["properties"]["part"]["sessionID"]`,
     `event["properties"]["info"]["sessionID"]`).
4. After all fixes, the viewer still does not update in real time. Exiting and
   re-entering the viewer correctly shows the new content, confirming that
   `fetch_session_content` works but SSE events are not triggering it.

**What can be further investigated.**

- **Verify the SSE connection is established**: add a brief status message or log
  line when `connect_and_subscribe` successfully receives an HTTP 200, vs failing
  silently on a non-2xx. Currently a 2xx opens the reader but there is no
  observable confirmation the connection is alive.
- **Verify events are arriving in the thread**: add a counter or log line each time
  a `data:` line is parsed, to confirm the thread is reading from the stream at all
  vs blocking in `lines.next()` indefinitely.
- **`ureq` vs long-lived streaming**: `ureq` 2.x is designed for request/response
  HTTP; long-lived streaming connections may behave differently. The `into_reader()`
  wrapper could be silently buffering or imposing undocumented limits. Alternative:
  switch the SSE thread to use `reqwest` (blocking feature) or raw `TcpStream` +
  manual HTTP/1.1 GET for the SSE endpoint.
- **Accept header**: opencode's SDK sets no explicit `Accept: text/event-stream`
  header. Verify whether the server requires it; add it if missing.
- **Manual verification**: run `curl -N http://127.0.0.1:4096/global/event` while
  octmux is processing a prompt and confirm events appear in the terminal. If they
  do, the endpoint is correct and the issue is in the Rust reader; if they don't,
  the endpoint or server has a different issue.
- **Threading**: confirm `sse_rx` is still `Some` (i.e., `stop_sse()` was not
  accidentally called) at the point when events should be arriving. A misfire of
  `Disconnected` on the channel would silently drop the receiver.

**Current mitigation.** v4 ships with `Ctrl-L` list refresh and enter/exit viewer
as the manual content-refresh path. Real-time streaming is de-prioritised; this
entry is the resolution target for a future patch once the root cause is confirmed
via the steps above.

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

Status: ✓ shipped — see Changelog 2026-05-25--08-30

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

Status: 🟡 partially implemented — see Changelog 2026-05-25--11-00

**Note:** Real-time SSE streaming into the viewer pane was not achieved despite
multiple fixes (see Open Questions → "2026-05-25 SSE live streaming"). The stage
shipped the SSE infrastructure and the following practical improvements:

- `Ctrl-L` in the main list to manually refresh the session list (picks up new
  sessions created after oc-history started).
- Entering and exiting the viewer correctly re-fetches and displays the latest
  session content, including any turns added since the viewer was last opened.
- Turn count in the list is synced when the viewer is entered or updated.

Real-time streaming in the viewer is de-prioritised and tracked in Open Questions.

### Assumptions

- v3 shipped; viewer is static-rendered with navigation and search.
- `src/opencode/client.rs` exposes basic HTTP operations; SSE is new in v4.

### Original goal (partially met)

When a session is open and its status is `busy`, the viewer follows new messages
and parts in real time via SSE. New content appends automatically; on
`session.idle` the viewer shows a "Completed" indicator.

**What shipped:** SSE infrastructure (`src/opencode/sse.rs`), session list refresh
(`Ctrl-L`), turn count sync, viewer re-entry as manual refresh path.

**What did not ship:** Real-time content streaming into the open viewer pane. See
Open Questions for investigation notes and next steps.

### Deliverables

Created: `src/opencode/sse.rs`

Modified:

- `src/tui/app.rs` (SSE channel, `Ctrl-L` reload, turn count sync)
- `src/tui/ui.rs` (`^L refresh list` hint in status bar)
- `docs/Changelog.md`; `docs/Implementation-plan.md`

### Handover notes for v5

- v5's workspace scope toggle triggers a new `list_sessions` call with
  `?directory=`; the SSE subscription (global) is unaffected.
- SSE streaming root cause should be resolved before or during v5; see Open
  Questions entry dated 2026-05-25.

---

## Stage v5 — Export from viewer (opencode-aware) + export.rs cleanup

Status: ✓ shipped — see Changelog 2026-05-25--14-07

### Assumptions

- v0..v4 shipped; viewer renders text, tools, reasoning, timing; within-viewer search works.
- `src/tui/export.rs` contains dead claude-history code (JSONL-based generators, claude types).
- Real session data lives in `ViewState.session_content: Option<OcSessionView>` with
  `Vec<MessageView>` and `Vec<ViewPart>` enum (Text, Reasoning, ToolCall, StepFinish).

### Goal

Pressing `e` in viewer mode exports the conversation (as currently rendered, respecting
tool/thinking toggles) to a file in one of 3 text formats. All dead claude-history code
in `export.rs` is deleted; the module is rewritten as a pure opencode-aware exporter
with ~150 lines, no `#[allow(dead_code)]`, no claude imports.

### In scope

- **3 export formats:**
  - `Ledger`: 9-char speaker column + "│" separator; text wrapped to 90 chars total.
  - `Plain`: "User:\n{text}" / "Assistant:\n{text}".
  - `Markdown`: "## User\n\n{text}" / "## Assistant\n\n{text}"; tools in fenced code; thinking as blockquote.
- **Display toggle-aware rendering:** respects current `ToolDisplayMode`, `show_thinking`, `show_timing`.
- **File + clipboard export:** `e` menu (3 options) + `y` menu for clipboard variant.
- **Filename generation:** `<sanitized-session-title>--<timestamp>.{txt|md}`.
- **Complete rewrite of `src/tui/export.rs`:**
  - Delete: all JSONL parsers, `ExportOptions`, `ExportResult`, `extract_message_text`, all
    claude-type matching, all helper functions for JSONL-based generators.
  - Keep: `copy_to_system_clipboard` (Linux platform utilities), `sanitize_filename`, `wrap_plain_text`,
    `append_ledger_block`, `LEDGER_WIDTH`.
  - Add: `ExportFormat::from_index(0..2)`, `.extension()`, `render_oc_export()`, format-specific renderers.
- **Update `EXPORT_OPTIONS` in `app.rs`:** 5 entries → 3 (remove JSONL and Operator dialogue).
- **Rewrite `perform_export()`:** use `render_oc_export()` instead of broken path logic.
- **Session title as filename:** `ViewState.custom_title` populated from session title on viewer entry.

### Out of scope

- Cross-session fuzzy search (v6).
- Session rename / workspace scope toggle (later).
- Per-message copy (deferred).

### Deliverables

Modified:

- `src/tui/export.rs` (completely rewritten: 1100 lines → ~150 lines; opencode-only)
- `src/tui/mod.rs` (add `mod export` declaration; re-export public functions)
- `src/tui/app.rs` (shrink `EXPORT_OPTIONS` to 3; rewrite `perform_export`; populate `custom_title`)
- `src/tui/ui.rs` (`render_export_menu` updated to 3-option list)
- `docs/Implementation-plan.md` (this section; marker flip)
- `docs/Changelog.md` (v5 entry)

### Tests

1. `cargo build --release` succeeds with no errors.
2. Enter a session; press `e` → menu shows 3 options (Ledger, Plain text, Markdown); no JSONL.
3. Press `1` (Ledger) → file created as `<session-title>--<date>.txt` in current directory.
4. Toggle tools off (`t` → Hidden), export → tool calls absent from the file.
5. Press `3` (Markdown) → `.md` file with `## User` / `## Assistant` headers.
4. `y` menu (clipboard) works for all 4 formats.
5. Export respects current `tool_display`, `show_thinking`, `show_timing` settings.
6. OperatorMarkdown exports dialogue only (no tools, no thinking).

### Handover notes for v6

- `copy_focused_message` is a stub; per-message copy is deferred.
- `ViewState.conversation_path` remains a field (not removed in this stage).
- Export feature is stable; next session-level feature is workspace scope / rename.

---

## Stage v5.1 — Double-Esc exit guard for list mode

Status: ✓ shipped — see Changelog 2026-05-25--21-38

### Assumptions

- v5 shipped; export feature complete.
- App struct initialized with Esc behavior logic in `handle_list_key`.

### Goal

Implement a graceful exit confirmation guard: pressing Esc with empty query
shows a status message "Press Esc again to exit" instead of quitting immediately.
Second Esc quits; any other key cancels the pending quit. Applies to both main
session listing and Tab-scoped project listing.

### In scope

- Add `esc_pending_quit: bool` field to `App` struct.
- Initialize field to `false` in all three constructors (`new`, `new_loading`, `new_single_file`).
- Rewrite Esc handling in loading branch of `handle_list_key` to implement
  double-Esc guard.
- Rewrite Esc handling in ready branch of `handle_list_key` to implement
  double-Esc guard (identical logic, two separate match arms).
- Reset `esc_pending_quit` at top of `handle_list_key` on any non-Esc key.

### Out of scope

- Ctrl+C exit guard (remains unconditional).
- Esc behavior in view mode (unchanged).
- Esc behavior in dialog overlays (unchanged).
- Tab/workspace_filter logic (unchanged).

### Deliverables

Modified:

- `src/tui/app.rs` (add field; initialize in constructors; rewrite Esc branches; reset on non-Esc key)
- `docs/Changelog.md` (v5.1 entry)
- `docs/Implementation-plan.md` (this section; marker flip)

### Tests

1. `cargo build --release` succeeds with no errors (this is a gate).
2. List mode, empty query: press Esc → status bar shows "Press Esc again to exit".
3. Press Esc again → app quits.
4. List mode, empty query: press Esc, then press any other key (e.g., `j`) → status message clears, behavior normal.
5. Workspace filter (Tab) active: repeat steps 2–4 in scoped session view.
6. List mode with non-empty query: press Esc → query clears (no double-Esc needed).

### Handover notes for v6

- Double-Esc guard is stable and correct; next session-level feature is
  cross-session search (open decision pending on index strategy).

---

## Stage v5.2 — External pager integration

Status: ✓ shipped — see Changelog 2026-05-26--11-12

### Assumptions

- v5.1 shipped; list mode and export features complete.
- `src/pager.rs::spawn_pager()` already exists, reading `$PAGER` or defaulting to `less -sCIR`.
- `Ctrl+V` in list mode triggers `Action::OpenInPager(PathBuf)` (already wired in `handle_list_key`).

### Goal

Implement the pager action so that pressing `Ctrl+V` on a highlighted session renders the conversation and opens it in the external pager. The TUI suspends, displays pager output, and resumes cleanly on pager exit.

### In scope

- New `open_text_in_pager(text: &str) -> io::Result<()>` function in `src/pager.rs`: spawns pager, pipes text via stdin, waits for exit.
- List-mode handler (~line 2890): fetch session via `opencode_client.fetch_session_content()`, render using current display toggles, drop `guard`, call `open_text_in_pager()`, re-create `guard`.
- Single-file mode handler (~line 3001): unreachable stub comment (view mode has no Ctrl+V handler).
- Terminal guard lifecycle: `drop(guard)` restores terminal, `guard = TerminalGuard::new()?` re-enters alternate screen.

### Out of scope

- Pager configuration beyond `$PAGER` env var.
- Per-pager option flags (those live in the env var itself).
- Resume / fork actions (deferred).

### Deliverables

Modified:

- `src/pager.rs` (add `open_text_in_pager`)
- `src/tui/app.rs` (implement list-mode handler, stub single-file handler)
- `docs/Implementation-plan.md` (this section; marker flip)
- `docs/Changelog.md` (v5.2 entry; frontmatter refresh)

### Tests

1. `cargo build --release` succeeds with no errors (this is a gate).
2. List mode: highlight a session, press `Ctrl+V` → TUI exits alternate screen, pager opens with rendered conversation, pressing `q` in less returns to TUI.
3. Render respects current display toggles (tool mode, thinking blocks).
4. Session with fetch failure → status message "Pager: fetch failed — {error}".
5. Pager spawn failure → logged to stderr "Pager error: {error}".

### Handover notes for v6

- Pager feature is complete and stable.
- Next session-level feature is cross-session search (open decision pending on index strategy).

---

## Stage v5.3 — Display model name per-turn

Status: ✓ shipped — see Changelog 2026-05-26--20-46

### Assumptions

- v5.2 shipped; pager feature complete.
- `MessageInfo` can be extended with model fields from opencode's HTTP envelope.
- `MessageView` and `viewer.rs` can be updated to display model labels.

### Goal

Replace the `[assistant]` role label in message headers with a hybrid label `[assistant - <modelID>]`, so model changes are visible turn-by-turn in the session viewer. User messages remain `[user]`.

### In scope

- Add `MessageModel` struct to capture nested model objects from user messages.
- Add `model`, `model_id`, `provider_id` fields to `MessageInfo`.
- Derive `model_label: Option<String>` in `fetch_session_content` (prefer flat fields, fall back to nested).
- Add `model: Option<String>` field to `MessageView`.
- Update viewer header rendering to emit `[assistant - <modelID>]` for assistant messages.

### Out of scope

- Per-message model tracking beyond display (no filtering, no statistics).
- Provider name display (only model ID).
- User message model display.

### Deliverables

Modified:

- `src/opencode/models.rs` (add `MessageModel`; extend `MessageInfo` and `MessageView`)
- `src/opencode/client.rs` (derive `model_label` in `fetch_session_content`)
- `src/tui/viewer.rs` (hybrid label rendering for assistant messages)
- `docs/Changelog.md` (v5.3 entry)
- `docs/Implementation-plan.md` (this section; marker flip)

### Tests

1. `cargo build --release` succeeds with no errors (gate).
2. Open a session with multiple assistant turns from different models → headers show `[assistant - claude-opus-4-7]`, etc.
3. User turns remain `[user]`.
4. Sessions with no model data → assistant turns show `[assistant]` (fallback).

### Handover notes for v6

- Model label display is stable; next session-level feature is cross-session search.

---

## Stage v5.4 — Fix session listing: switch to v2 /api/session

Status: ✓ shipped — see Changelog 2026-05-27--14-33

### Assumptions

- v5.3 shipped; model-name display complete.
- opencode server at 192.168.1.95:4096 exposes `GET /api/session` (v2 endpoint).

### Goal

Fix a silent data-loss bug: `list_sessions()` used `GET /session` (v1) which filters by server cwd, causing oc-history to only show 59 of 89 sessions. Switch to `GET /api/session` (v2 paginated) which returns all sessions regardless of cwd.

### In scope

- `V2SessionList`, `V2Cursor`, `V2SessionItem` types in `models.rs`.
- Rewrite `list_sessions()` in `client.rs` to walk the v2 cursor loop (1000-iteration cap, PAGE_LIMIT=100).
- `directory` reconstruction from v2 `path` (prepend `/`; use `/` for empty path).
- `urlencoding = "2"` added to `Cargo.toml`.

### Out of scope

- Removing the legacy `Session` struct (Option B in fix doc) — follow-up.
- `probe_health()` fallback update — cosmetic, deferred.

### Deliverables

Modified:
- `Cargo.toml` (urlencoding dep)
- `src/opencode/models.rs` (V2 types)
- `src/opencode/client.rs` (rewrite list_sessions)
- `docs/Changelog.md` (v5.4 entry)
- `docs/Implementation-plan.md` (this section)

### Tests

1. `cargo build --release` succeeds with no errors (gate).
2. TUI shows 89 sessions (was 59) against Server 2 deployment.
3. SoHoAI project sessions and octmux-launched sessions visible.
4. Delete a session → still works (DELETE /session/{id} is unchanged).

### Handover notes for v6

- Session listing is now complete and correct.
- The legacy `Session` struct still carries unused `version` / `parent_id` fields — clean up in v1 (or a dedicated follow-up, not v6).

---

## Stage v5.5 — Session-ID positional argument

Status: ✓ shipped — see Changelog 2026-05-27--14-42

### Assumptions

- v5.4 shipped; model-name display and session listing complete.
- `src/main.rs::run()` contains the primary TUI loop; new dispatch path can be inserted before it.
- `src/tui/render_conversation()` API is available and works with `Option<&OcSessionView>`.

### Goal

Add a positional `SESSION` argument to `oc-history` that accepts a bare session ID (`ses_...`) or opencode URI (`opencode://ses_...`), skips the TUI, fetches the session via HTTP, renders it with current display toggles, and opens the result in the external pager. Invalid IDs exit immediately with a clear error.

### In scope

- `src/cli.rs`: add `session: Option<String>` positional field to `Args` struct.
- `src/cli.rs`: add `parse_session_id(input: &str) -> Result<String, String>` validator function.
- `src/main.rs`: add early dispatch block after subcommand check; new `run_session_pager()` function.
- `run_session_pager()` logic: fetch session, build `RenderOptions` from CLI flags, render, emit ANSI-escaped text to pager.

### Out of scope

- TUI customization for pager mode (content_width, etc.).
- Cross-session features; only single-session pager.

### Deliverables

Modified:

- `src/cli.rs` (add `session` field and `parse_session_id()` function)
- `src/main.rs` (add direct pager dispatch and `run_session_pager()` function)
- `docs/Implementation-plan.md` (this section; marker flip)
- `docs/Changelog.md` (v5.5 entry; frontmatter refresh)

### Tests

1. `cargo build --release` succeeds with no errors.
2. `oc-history ses_invalid` → exit 1 with clear error about session ID format.
3. `oc-history opencode://ses_<id>` (valid ID, server running) → pager opens with session content.
4. `oc-history <id>` with `--show-tools` → pager renders with full tool calls.
5. Display toggles (`--no-tools`, `--show-thinking`) respected in output.

### Handover notes for v6

- Session-ID argument is stable; next feature is cross-session search with local index.
- Pager mode and TUI mode are now separate code paths; future enhancements (e.g., output format) can target either independently.

---

## Stage v5.6 — Pager-to-TUI cursor continuation

Status: ✓ shipped — see Changelog 2026-05-27--15-15

### Assumptions

- v5.5 shipped; pager mode is stable and functional.
- The TUI `run_with_loader()` function can accept additional parameters without breaking the loader message loop.
- App struct's `filtered` index correctly maps to `conversations` and session IDs are available via `c.id`.

### Goal

After the user quits the pager (launched via `oc-history <session_id>`), the process does not exit. Instead, the TUI session list opens with the cursor positioned on the session that was just viewed in the pager. This provides a seamless transition from pager back to list view.

### In scope

- Restructure `src/main.rs::run()` to capture the session ID from pager mode and pass it as an optional parameter to the TUI loader.
- Add `pre_select_id: Option<&str>` parameter to `src/tui/app.rs::run_with_loader()`.
- Implement cursor positioning logic in the `LoaderMessage::Done` arm: search `app.filtered` for a session matching the pre-select ID and set `app.selected` if found.
- Implement the same cursor positioning logic in the `Err(Disconnected)` arm for the fallback case.

### Out of scope

- Cursor positioning outside the loaded list (no special handling if session is filtered by workspace tab).
- Any changes to pager mode itself; pager-mode logic remains unchanged.

### Deliverables

Modified:

- `src/main.rs` (restructure `run()` to flow through pager then TUI; pass pre-select ID)
- `src/tui/app.rs` (add `pre_select_id` parameter; implement cursor positioning in loader arms)
- `docs/Implementation-plan.md` (this section; marker flip)
- `docs/Changelog.md` (v5.6 entry; frontmatter refresh)

### Tests

1. `cargo build --release` succeeds with no errors (this is a gate).
2. `oc-history ses_<valid_id>` → pager opens, user quits → TUI list opens with cursor on that session.
3. If the session is filtered out (e.g., workspace tab active), cursor defaults to first visible session.
4. Cursor correctly identifies the session by comparing `c.id` against the passed ID.
5. Without a positional argument, TUI opens normally (backward compatible).

### Handover notes for v7

- Pager-to-TUI transition is stable; the main feature loop is complete for v5.x.
- Next session is cross-session search (v6), pending architecture decision on indexing strategy.

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
