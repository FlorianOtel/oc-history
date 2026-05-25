---
title: "oc-history — Changelog"
created_at: 2026-05-24--09-45
created_by: Florian Otel florian.otel@gmail.com
updated_by: Claude Code (Claude Haiku 4.5)
updated_at: 2026-05-25--13-34
context: >
  Changelog -- Feature implementation changelog for 'oc-history' project.
  Pre-fork (upstream raine/claude-history) history is preserved as an
  appendix at the bottom for provenance; only entries above the appendix
  are oc-history's own log.
---

# Changelog pre-implementation checklist - Read this first

When implementing a change:

1. Read this doc top-to-bottom, paying attention to the most recent log
   entry — it carries forward notes from the previous implemention plan that the spec
   below may not capture.
2. Implement only the deliverables and files listed for the current change / implementation plan stage.
   Do not pull work forward from later implementation stages.
3. Run the change's manual verification steps. All must pass.

When finishing a change:

1. Add a new entry at the top of "Implementation log" with today's
   `YYYY-MM-DD--HH-MM` timestamp. Each entry must include:
   - **Implemented by:** `<agent name (model)> — YYYY-MM-DD--HH-MM`
   - **Commit(s):** `hash1`, `hash2` — all hashes comma-separated on one line
2. Check the implementation plan in file "Implementation-plan.md", and mark the corresponding stage in the the parent plan to `✓ shipped — see log
   YYYY-MM-DD--HH-MM`.
3. Refresh `updated_by` and `updated_at` in the frontmatter.
4. Commit with `feat(oc-history): Changelog N — <short title>`.

---

## Changelog (reverse chronological — newest at top)

## v5 — Export from viewer (opencode-aware) + export.rs cleanup (2026-05-25--13-34)

- **Implemented by:** Claude Code (Claude Haiku 4.5) — 2026-05-25--13-34
- **Commit(s):** (pending)

### What shipped

Stage v5 completes the export feature for opencode sessions. Pressing `e` in the viewer
opens a 4-option export menu; selecting an option (or pressing `1`–`4` for direct choice)
exports the current conversation respecting tool/thinking toggle settings.

**4 export formats:**
- **Ledger**: 9-character speaker column with "│" separator; text wrapped to 90 chars total.
- **Plain**: "User:\n{text}" / "Assistant:\n{text}" format with conditional tool/thinking/timing blocks.
- **Markdown**: "## User\n\n{text}" headers; tools and thinking in fenced code/blockquote blocks.
- **Operator dialogue**: Markdown format showing only user/assistant text (no tools, no thinking).

**Export destinations:**
- File: saved to `<sanitized-title>--<YYYY-MM-DD--HH-MM>.{txt|md}` in current directory.
- Clipboard: via `y` menu variant (same 4 formats).

**File cleanup:**
- `src/tui/export.rs`: completely rewritten from 1100 lines to ~150 lines.
  - **Deleted**: all JSONL-based code, `ExportOptions` struct, `ExportResult`, `extract_message_text`,
    all claude type matching, all JSONL generators.
  - **Kept**: `copy_to_system_clipboard` (Linux platform utilities), `sanitize_filename`, `wrap_plain_text`,
    `append_ledger_block`.
  - **Added**: `ExportFormat::from_index()`, `.extension()`, `render_oc_export()`, 4 opencode-aware format renderers.
- `src/tui/mod.rs`: added `mod export` declaration; re-exported `ExportFormat`, public helpers.
- `src/tui/app.rs`: removed JSONL export option; `EXPORT_OPTIONS` shrunk from 5 to 4 entries; 
  removed `KeyCode::Char('5')` arm; rewrote `perform_export()` to use `render_oc_export()`; 
  stubbed `copy_focused_message()` (per-message copy deferred).

### Files changed

- `src/tui/export.rs` — complete rewrite (1100 → ~150 lines)
- `src/tui/mod.rs` — added export module declaration
- `src/tui/app.rs` — export integration, menu shrink, per-message copy stub
- `docs/Implementation-plan.md` — Stage v5 status flipped to shipped; Open Questions dates updated
- `docs/Changelog.md` — this entry

### Verification

1. `cargo build --release` succeeds with no errors.
2. Enter a session; press `e` → 4-option menu displays.
3. Press `1` (Ledger) → file created with correct name and ledger format.
4. Press `y` (yank menu) → clipboard export works for all formats.
5. Toggle tool/thinking/timing; export respects current settings.
6. OperatorMarkdown shows dialogue only, no tools or thinking.

---

## v4 — Stage close: SSE streaming de-prioritised (2026-05-25--11-00)

- **Implemented by:** Claude Code (Claude Sonnet 4.6) — 2026-05-25--11-00
- **Commit(s):** a2f781b

### What shipped

Stage v4 is marked **partially implemented**. Real-time SSE streaming into the open
viewer pane was not achieved despite the SSE infrastructure being in place and multiple
fixes applied (wrong endpoint URL, wrong sessionID JSON path). The root cause is
unresolved and tracked in `docs/Implementation-plan.md → Open Questions (2026-05-25)`.

**Practical workarounds available:**
- Enter and exit the viewer to pick up the latest session content (re-fetches via HTTP).
- `Ctrl-L` in the main list refreshes the session list, picking up sessions created
  after oc-history started and updating turn counts.

**Documentation updates:**
- `docs/Implementation-plan.md`: Stage v4 status flipped to `🟡 partially implemented`;
  new Open Questions entry documents what was tried, what failed, and next investigation
  steps.

### Files changed

- `docs/Implementation-plan.md` — stage status + Open Questions entry
- `docs/Changelog.md` — this entry

---

## v4 hotfix — SSE correct endpoint URL (2026-05-25--10-45)

- **Implemented by:** Claude Code (Claude Sonnet 4.6) — 2026-05-25--10-45
- **Commit(s):** a2f781b

### What shipped

- **One-line fix (`sse.rs`)**: SSE endpoint was `GET /sse/global/event`; the correct
  opencode endpoint (matching the SDK and octmux) is `GET /global/event`. The
  `/sse/` prefix does not exist, causing every connection to fail with a non-2xx
  response and silently suppress all live updates.

### Files changed

- `src/opencode/sse.rs` — URL corrected

---

## v4 hotfix — SSE session ID path + turn count sync + Ctrl-L reload (2026-05-25--10-15)

- **Implemented by:** Claude Code (Claude Sonnet 4.6) — 2026-05-25--10-15
- **Commit(s):** a2f781b

### What shipped

- **Critical bug fix (`sse.rs`)**: opencode SSE events nest the session ID inside
  `event.properties.*` — not at top level. The original filter looked up `event.sessionID`
  (always absent), silently dropping every event. Fixed by per-type extraction:
  `message.part.delta/session.idle` → `properties.sessionID`,
  `message.part.updated` → `properties.part.sessionID`,
  `message.updated` → `properties.info.sessionID`.
- **Turn count sync (`app.rs`)**: `apply_sse_update` and `enter_view_mode` now count
  assistant messages from the freshly fetched `OcSessionView` and write the result
  back to `Conversation.turn_count` in the list, keeping the list in sync.
- **Ctrl-R reload (`app.rs` + `ui.rs`)**: `Ctrl-R` in list mode calls `reset_for_reload()`
  and re-spawns `load_sessions_streaming`. Status bar now shows `^R reload` just before
  `^H help`.

### Files changed

- `src/opencode/sse.rs` — sessionID path fix
- `src/tui/app.rs` — turn count sync, `ReloadSessions` action, `reset_for_reload`, Ctrl-R key
- `src/tui/ui.rs` — `^R reload` hint in status bar

---

## v4 — Live follow via SSE (2026-05-25--09-36)

- **Implemented by:** Claude Code (Claude Haiku 4.5) — 2026-05-25--09-36
- **Commit(s):** a2f781b

### What shipped

- New `src/opencode/sse.rs`: background thread subscribes to `GET /sse/global/event`;
  filters by session ID via JSON inspection; sends normalised `SseEvent` variants
  (`ContentChanged`, `SessionIdle`, `Reconnecting`, `Failed`) to main loop via `mpsc`.
- `App.sse_rx`: SSE receiver stored on `App` (not `ViewState`) to avoid `Clone` conflict.
- `ViewState.live_follow`: auto-scrolls to bottom on SSE updates unless user scrolled up;
  re-engages on `G`/`End`.
- Main event loop polls SSE at 100 ms when viewer is active.
- Clean teardown: dropping `sse_rx` signals the background thread to exit.
- Status messages for `session.idle`, reconnect attempts, and failures.

### Files changed

- `src/opencode/sse.rs` — new SSE subscriber module
- `src/opencode/mod.rs` — re-export `SseEvent`
- `src/tui/app.rs` — add `live_follow` field to `ViewState`, `sse_rx` field to `App`,
  `start_sse_subscriber()`, `stop_sse()`, `sse_active()`, `apply_sse_update()` methods;
  wire SSE subscriber start/stop on enter/exit view mode; add live-follow toggle to
  scroll actions; poll SSE in main event loop; update poll_timeout logic for SSE polling.
- `docs/Implementation-plan.md` — v4 status marker
- `docs/Changelog.md` — this entry

### Manual verification

- `cargo build --release` succeeds, 78 warnings (pre-existing), 0 errors.
- Open a session in viewer mode.
- From another terminal, send a prompt to opencode; text streams into the viewer.
- Viewer auto-scrolls to bottom as new content arrives.
- Scroll up manually; new content arrives but does not auto-scroll (live_follow disables).
- Press `G` to jump to bottom; live_follow re-enables.
- Mark the session as idle in opencode; viewer shows "Session completed" message.

---

## v3 — Within-viewer navigation + search: wire n/N (2026-05-25--08-30)

- **Implemented by:** Claude Code (Claude Haiku 4.5) — 2026-05-25--08-30
- **Commit(s):** 212e261

### What shipped

**Match cycling keybindings:**
- v3 completes within-viewer search by wiring `n`/`N` match cycling to the existing `next_search_match` and `prev_search_match` methods.
- `/` (forward search) and `?` (backward search) were already working from v2 infrastructure.
- `J`/`K`/`[`/`]` message navigation was already wired in v2.
- `message_ranges` population in the renderer was already implemented in v2.
- Only missing piece was the key-binding arms in `handle_view_key` — now added:
  - `KeyCode::Char('n')` → `self.next_search_match(viewport_height)`
  - `KeyCode::Char('N')` → `self.prev_search_match(viewport_height)`

### Files changed

- `src/tui/app.rs` — added `n`/`N` match-cycling arms to `handle_view_key`
- `docs/Implementation-plan.md` — v3 status marker
- `docs/Changelog.md` — this entry

### Manual verification

- `cargo build --release` succeeds, 79 warnings (pre-existing dead code), 0 errors.
- Open a session in viewer mode.
- Press `/` to start a forward search, type a query, press Enter.
- Press `n` to cycle to the next match.
- Press `N` to cycle to the previous match.
- Matches are highlighted and the viewer scrolls to show the current match.

---

## v2 — Tool calls, thinking blocks, timing markers (2026-05-25--10-15)

- **Implemented by:** Claude Code (Claude Haiku 4.5) — 2026-05-25--00-00; line-wrap hotfix Claude Code (Claude Sonnet 4.6) — 2026-05-25--10-15
- **Commit(s):** 7c9dc1b, e4e420f

### What shipped

**View-layer part representation:**
- `ViewPart` enum added (`src/opencode/models.rs`): non-serde view-layer type constructed by JSON value inspection. Variants: `Text(String)`, `Reasoning(String)`, `ToolCall { name, call_id, input, output, status }`, `StepFinish { cost, input_tokens, output_tokens }`.
- `MessageView.text_parts: Vec<String>` replaced with `parts: Vec<ViewPart>` to enable rich part dispatch.
- `MessageEnvelope.parts` remains `Vec<serde_json::Value>` (per v0 architectural constraint).

**Part extraction and dispatching:**
- `Client::fetch_session_content` rewritten (`src/opencode/client.rs`): iterates over raw JSON parts and dispatches by `part["type"]` to extract text, reasoning, tool calls, and step-finish markers.
- Tool call extraction: `tool` / `callID` / `state` → `ViewPart::ToolCall { name, call_id, input, output, status }` where output is populated only if status is `completed`.
- Step-finish extraction: `time.cost` / `time.tokens.input|output` → timing markers; unknown part types (`step-start`, etc.) silently dropped.

**Renderer updates:**
- `render_oc_session` signature unchanged (`src/tui/viewer.rs`), but now processes `msg.parts` instead of `msg.text_parts`.
- New `render_part` function: per-variant rendering logic honoring `options.tool_display`, `options.show_thinking`, `options.show_timing`.
- `ViewPart::Text` → normal lines (unchanged from v1).
- `ViewPart::Reasoning` → dim `[thinking]` header + dim lines if `show_thinking` is true; silent skip otherwise.
- `ViewPart::ToolCall`:
  - `Hidden` mode: no output.
  - `Truncated` mode: dim `▶ header` line + one truncated output line (120 chars, dim) if completed.
  - `Full` mode: dim `▶ header` line + body lines (dim) + all output lines (dim) if completed.
- `ViewPart::StepFinish` → dim timing line `  ↳ {input}↑ {output}↓ tokens` with cost appended if Some: `, ${:.4}` (only if `show_timing` is true).

**Tool formatting enhancements:**
- `format_tool_call` now normalizes tool names: lowercase first character capitalized for case-insensitive matching (e.g. `bash` → `Bash`).
- New `format_tool_output(output: &Value, truncate: bool) -> String` function: handles `Value::String`, `Value::Null`, and JSON serialization; truncates to 120 chars if requested.
- Tool output rendering in viewer calls `tool_format::format_tool_output` for consistent output display across truncated and full modes.

### Files changed

- `src/opencode/models.rs` — `ViewPart` enum; `MessageView.text_parts` → `MessageView.parts`
- `src/opencode/mod.rs` — re-export `ViewPart`
- `src/opencode/client.rs` — `fetch_session_content` with full part dispatch
- `src/tool_format.rs` — tool name normalization; `format_tool_output` function
- `src/tui/viewer.rs` — complete rewrite; `render_part` function; output formatting integration
- `docs/Implementation-plan.md` — v2 status marker
- `docs/Changelog.md` — this entry

### Line-wrap hotfix (commit e4e420f — back-fixes v1 as well)

Long lines in the viewer were not wrapped at the terminal width — text overflowed for both user/assistant content (present since v1) and the new tool/reasoning output (v2). Fixed by adding a `wrap_into_lines(text, width, style)` helper in `src/tui/viewer.rs` using the existing `textwrap` crate (0.16). All text pushes in `render_part` now go through this helper, which wraps lines longer than `options.content_width` while preserving existing line breaks. The fix covers all part types: `Text`, `Reasoning`, tool headers, tool bodies, and tool output.

This is a back-fix for v1 as well: the v1 text-only viewer had the same overflow bug; the same `render_part` path now handles it.

### Manual verification

- `cargo build --release` succeeds, 79 warnings (pre-existing dead code), 0 errors.
- Toggles `t` (tool display) / `T` (thinking) / `i` (timing) produce visible changes in viewer.
- Tool calls display with truncated output by default; `t` cycles to full (all output lines) and hidden (no tool lines).
- Thinking blocks appear (dim) only when `T` is pressed; hidden by default.
- Timing markers (`  ↳ input↑ output↓ tokens, $cost`) appear only when `i` is pressed; hidden by default.
- Long lines (text, tool output, reasoning) wrap at terminal width instead of overflowing.

---

## v1 — Session content viewer (text-only) (2026-05-24--23-11)

- **Implemented by:** Claude Code (Claude Haiku 4.5) — 2026-05-24--22-49; regression fix Claude Code (Claude Sonnet 4.6) — 2026-05-24--23-11
- **Commit(s):** 1d80018

### What shipped

**Opencode message models:**
- `MessageTime { created: i64, completed: Option<i64> }` added; `MessageInfo.time: Option<MessageTime>` added.
- `MessageEnvelope.parts` stays as `Vec<serde_json::Value>` — typed Part enum was attempted but serde's `#[serde(other)]` unit variant cannot absorb extra fields (`id`, `sessionID`, etc.) on internally-tagged objects; JSON value inspection is used instead (see regression note below).
- `OcSessionView { session_id, messages: Vec<MessageView> }` and `MessageView { role, created, text_parts }` added as construction-only types.

**Session content fetching:**
- `Client::fetch_session_content(session_id)` added: calls `list_messages`, extracts text parts by checking `part["type"] == "text"` on raw `serde_json::Value`, maps to `MessageView`.
- Timestamps from `envelope.info.time.created` (JS milliseconds since epoch).

**Viewer implementation:**
- `render_conversation` signature changed from `(path, options)` to `(content: Option<&OcSessionView>, options)`, delegating to `viewer::render_oc_session`.
- `src/tui/viewer.rs` rewritten (~80 lines): role headers (bold label + dim timestamp formatted as `YYYY-MM-DD HH:MM`), text lines, blank separators, `MessageRange` list.

**Application layer:**
- `ViewState.session_content: Option<OcSessionView>` added.
- `enter_view_mode` replaced: fetches content synchronously, constructs `ViewState`, calls `re_render_view`. Error shown in status bar on fetch failure.
- Call sites updated to pass `viewport_height` and `opencode_client`.

**Dead code cleanup:**
- Deleted: `src/claude.rs`, `src/display.rs`, `src/history/{loader,parser,cache,global_log}.rs` (6 files, ~3,500 lines).
- Kept `src/history/path.rs` — still needed for workspace filtering (`is_same_project`, `format_short_name_from_path`).

### Regression fix (same commit)

Initial Actor implementation generated a typed `Part` enum without `#[serde(tag = "type")]`
(external tagging), causing every `list_messages` call to fail — "invalid type string
`prt_<hash>`, expected unit". Root cause: serde's `#[serde(other)]` on a unit variant cannot
absorb the extra fields present on all opencode Part objects. Fixed by reverting to
`Vec<serde_json::Value>` for `parts` and extracting text parts via JSON value inspection.

### Files changed

- `src/opencode/models.rs` — `MessageTime`, `OcSessionView`, `MessageView`; parts stay `Vec<Value>`
- `src/opencode/mod.rs` — re-export `OcSessionView`, `MessageView`
- `src/opencode/client.rs` — `fetch_session_content` with JSON-value text extraction
- `src/tui/mod.rs` — `render_conversation` signature change
- `src/tui/viewer.rs` — complete rewrite for opencode session rendering
- `src/tui/app.rs` — `ViewState.session_content`, `enter_view_mode` implementation
- `src/main.rs` — remove `mod claude;`
- `src/history/mod.rs` — remove `mod global_log;`
- `docs/Implementation-plan.md` — v1 status marker
- `docs/Changelog.md` — this entry

### Manual verification

- `cargo build --release` succeeds, 91 warnings, 0 errors.
- With opencode on 4096: Enter on any session → viewer with role headers and text content.
- j/k, Page-Down, gg/G navigate the viewer; Esc returns to list.
- Empty session → "No messages in this session." placeholder.
- "0 turns" regression resolved: list correctly counts turns after the parts fix.

---

## v0.5 — per-project session listing (TAB title-scoped filter) (2026-05-24--20-33)

- **Implemented by:** Claude Code (Claude Sonnet 4.6) — 2026-05-24--20-33
- **Commit(s):** feea9fe

### What shipped

**Project HTTP layer:**
- `Project` model added (`src/opencode/models.rs`): `id`, `worktree`, `vcs_dir`, `vcs`.
- `list_projects()` added to HTTP client (`src/opencode/client.rs`): `GET /project`.
- `loader.rs` fetches projects at startup, builds `projectID → Project` map, and uses `project.worktree` as `conv.project`; derives `project_name` (last path segment) from it.

**Search fix (side-effect):**
- `search_text_lower` was `String::new()` for all sessions in v0 (stub). Fixed: now populated from `title + project_short`. Keyword search against session titles now works correctly.

**TAB filter — title-based grouping:**
- In this deployment all sessions share `projectID='global'` (opencode global mode) with identical `project.worktree='/'`, making directory-based project differentiation impossible. The filter therefore groups by exact `conv.title` match — a pragmatic pivot that works well in practice (the same tasks tend to share identical titles across sessions).
- `toggle_workspace_filter()` pins `current_project_dir_name` to the highlighted session's `.title` on TAB-on; clears on TAB-off.
- `update_filter()` workspace branch: exact match on `conv.title`.
- Search worker workspace branch: same predicate, so search and filter compose correctly.
- `has_project_context()` returns `!self.conversations.is_empty()` — Tab·All / Tab·Prj indicator appears as soon as sessions load.
- `current_project_name()` accessor added; UI search prompt renders the pinned title when filter is active (e.g. `SoHoAI project overview ❯`).

### Files changed

- `src/opencode/models.rs` — `Project` struct
- `src/opencode/client.rs` — `list_projects()`
- `src/opencode/mod.rs` — re-export `Project`
- `src/opencode/loader.rs` — project map fetch, `project_worktree`/`project_name`/`search_text_lower`
- `src/tui/app.rs` — `toggle_workspace_filter`, `update_filter`, search worker branch, `has_project_context`, `current_project_name`
- `src/tui/ui.rs` — search prompt with pinned title
- `docs/Implementation-plan.md` — v0.5 stage entry (updated to reflect actual implementation)
- `docs/Changelog.md` — this entry
- `docs/Stage-v05.md` — planning doc (new, untracked before this commit)

### Manual verification

Tested against opencode endpoint at 127.0.0.1:4096 (97 sessions, all under global project).
- `cargo build --release` clean.
- Typing a session title keyword narrows the list correctly (search fix verified).
- Highlighting a "SoHoAI project overview" session → TAB → list shows all 6 matching-title sessions; indicator shows `Tab·Prj`; prompt shows `SoHoAI project overview ❯`.
- TAB again → full list restored; indicator shows `Tab·All`.

---

## v0 — Bare list + safe delete (2026-05-24--11-45)

- **Implemented by:** Claude Code (Claude Opus 4.7 1M, orchestrating Sonnet 4.6 Planner / Haiku 4.5 Actor / Sonnet 4.6 Reviewer via the /brain pipeline) — 2026-05-24--11-45
- **Commit(s):** _pending_ (this entry's own commit)

### Delivered

- HTTP client layer against the opencode endpoint (`list_sessions` / `list_messages` / `delete_session` / `probe_health`) under `src/opencode/` using `ureq`.
- List TUI with six v0 columns: title · turns · project · started · cost · tokens. Sorted by `time.updated` desc. Title falls back to `ses_<7>` if `session.title` is empty (strip-`ses_` then take 7).
- Safe delete: existing confirmation dialog → `DELETE /session/{id}` → result surfacing (Deleted / NotFound / Refused) → list refresh.
- Binary renamed `claude-history` → `oc-history`; `--endpoint` flag + `OPENCODE_BASE_URL` env var; default `http://127.0.0.1:4096`.
- Startup health probe (`GET /health` with `GET /session` fallback); hard error on unreachable endpoint with explicit URL + override hint.
- Enter / Resume / Export / Copy / Select actions stubbed with "deferred to later stage" status messages; no silent exits.
- `docs/Implementation-plan.md` created (multi-stage plan with top-of-file Open Questions section — first entry: cross-session fuzzy search under pure-HTTP).
- Project `CLAUDE.md` created with the post-review release-build rule and project-specific conventions.
- Changelog filename normalized to `docs/Changelog.md` (mixed case); upstream raine/claude-history history preserved as appendix at the bottom for provenance.

### Stage marker

`docs/Implementation-plan.md` → Stage v0 marked `✓ shipped — see Changelog 2026-05-24--11-45`.

### Manual verification (operator action remaining)

- `cargo build --release` — **clean** (92 warnings, 0 errors). Verified.
- `oc-history --version` → `oc-history 0.1.0`. Verified.
- `oc-history --help` → mentions `oc-history`, `--endpoint`, `OPENCODE_BASE_URL`. Verified.
- `oc-history --endpoint http://127.0.0.1:9999` → exits 1 with "Cannot reach opencode at http://127.0.0.1:9999 / Start opencode in headless mode or set --endpoint / OPENCODE_BASE_URL to the correct address." Verified.
- _Remaining manual checks (require opencode server running on 4096):_ list shows six columns sorted newest-first; `d`+`y` removes a throwaway session; Enter shows "Session viewer: deferred to v1" status.

### Notes

- Cargo edition changed from `2024` to `2021` to match the project's Rust toolchain (let-chain syntax adjusted across ~60 sites).
- Cargo package version reset to `0.1.0` to reflect the fork's new identity (upstream tagged `0.1.53` at fork point — see appendix).
- Dead-code submodules (`src/claude.rs`, `src/display.rs`, `src/history/{loader,parser,path,cache,global_log}.rs`, parts of `src/tui/{search,viewer,export}.rs`) retained on disk with stub / `#[allow(dead_code)]` shims; full cleanup deferred to v1.
- One non-blocking cosmetic noted by Reviewer for v1 cleanup: stale module-level `const LINES_PER_ITEM: usize = 3;` at `src/tui/ui.rs:25` is shadowed by the function-local `2`, and `src/tui/app.rs:2309` uses the stale `3` for mouse-click-to-row mapping. Keyboard navigation is unaffected; mouse-click row targeting is off by one.

---

# Appendix — Pre-fork upstream history (raine/claude-history)

Everything below was inherited verbatim from the upstream `claude-history` project (https://github.com/raine/claude-history) at the fork point (v0.1.53, April 2026). It documents the codebase oc-history was lifted from and is preserved here for provenance only; entries below do **not** describe oc-history work.

## Unreleased

- Write tool calls now show the file content they write in the conversation
  viewer (when tool output is visible) and in all export formats — previously
  only the filename was shown, silently dropping the written content
- The OperatorMarkdown / simplified export and the inline TUI viewer (when tool
  display is off) now include Write tool content as Claude output with a
  `Written to <file>` attribution, so diagnosis documents and plans written by
  the LLM are never silently hidden

## v0.1.53 (2026-04-17)

- Conversation viewer no longer jumps to unrelated content when toggling tool
  output (`t`), thinking (`T`), or timing (`i`), or when resizing the terminal —
  the viewport now stays anchored to the message you were reading

## v0.1.52 (2026-04-17)

- Mouse wheel scrolling in both the search result list and conversation viewer,
  and click-to-open on rows in the search result list (note: enabling mouse
  capture may interfere with click-drag text selection in some terminals — hold
  Shift, or Option on macOS, to bypass)
- Search results now show the selected position (e.g. current/total) so it's
  easier to tell where you are in the list
- Improved search snippet previews — the context line now prefers locations
  where query terms appear adjacent, instead of locking onto boilerplate matches
  that happen earlier in the conversation
- Fixed search ranking missing adjacent-phrase matches when the phrase was
  wrapped in markdown punctuation like `**media pipeline**`
- Added a Nix flake for installation on Nix systems

## v0.1.51 (2026-03-29)

- Improved search ranking — results now score matches by where they appear
  (title, project, summary, or message body), so exact project and title matches
  rank above incidental mentions in conversation text
- Search freshness scoring uses smooth decay instead of sharp cutoffs, giving
  more natural ranking between recent and older conversations

## v0.1.50 (2026-03-27)

- Pressing Esc now clears the search input first — a second Esc quits the app

## v0.1.49 (2026-03-24)

- Faster startup with per-project binary caching of parsed conversations — only
  changed files are re-parsed on subsequent launches
- Reduced memory usage by streaming JSONL lines instead of loading entire files
  into memory

## v0.1.48 (2026-03-24)

- Fixed search missing content in long conversations due to a 256K character
  truncation limit — all conversation text is now fully searchable
- Added Windows support — compilation and home directory resolution now work
  correctly on Windows ([#26](https://github.com/raine/claude-history/pull/26))

## v0.1.47 (2026-03-22)

- Fixed conversations that only contain skill invocations (e.g. `/consult`,
  `/commit`) being incorrectly filtered out as empty sessions

## v0.1.46 (2026-03-21)

- Fixed the screen freezing when holding down arrow keys or j/k to scroll — the
  view now redraws smoothly during key repeat instead of jumping when the key is
  released ([#25](https://github.com/raine/claude-history/issues/25))

## v0.1.45 (2026-03-20)

- Skill invocation prompts (e.g. from `/consult`, `/commit`) are now hidden from
  search results and shown as a concise description in the conversation viewer
  instead of displaying the full expanded prompt text

## v0.1.44 (2026-03-17)

- Added support for `CLAUDE_CONFIG_DIR` environment variable — users with custom
  Claude config directories can now use claude-history without workaround
  ([#24](https://github.com/raine/claude-history/issues/24))

## v0.1.43 (2026-03-14)

- Added `claude-history update` command for self-updating the binary directly
  from GitHub releases

## v0.1.42 (2026-03-13)

- Subagent messages are now included in J/K message navigation and single
  message copy
- Plain text mode (`--plain`) now supports pager output
- Fixed `--no-color` flag being ignored in normal (non-render) display mode
- Fixed text wrapping for CJK characters and emoji that occupy two terminal
  columns but were counted as one, causing text to overflow
- Deleting a session in the TUI (`Ctrl+X`) now removes the full session
  directory, not just the transcript file
- Fixed a potential crash when deleting a conversation while a search was
  in-flight
- Fixed conversations opened by UUID not showing project name or matching
  workspace filter
- `--fork-session` now requires `--resume` and shows an error if used alone
  instead of being silently ignored

## v0.1.41 (2026-03-13)

- Workspace filter now includes conversations from git worktrees of the same
  project, so sessions started in workmux worktrees appear alongside the main
  project's sessions
- Search result counter now shows the count relative to the current scope
  (project or global) instead of always showing the total

## v0.1.40 (2026-03-13)

- Search typing is now smoother — search runs in a background thread so
  keystrokes no longer block the UI, especially with large history
- Global view is now the default — all conversations are shown on launch instead
  of only the current workspace's sessions
  ([#21](https://github.com/raine/claude-history/pull/21))
- Added `Tab` key to toggle between global and workspace-only view in the TUI
  ([#21](https://github.com/raine/claude-history/pull/21))
- Added `-L`/`--local` flag to start with workspace filter active
- Deprecated `--global`/`-g` flag and `global` config option — global is now the
  default behavior

## v0.1.39 (2026-03-13)

- Added `--delete` flag to remove a session by its ID directly from the command
  line, e.g. `claude-history --delete <session-id>`
  ([#23](https://github.com/raine/claude-history/issues/23)
- Added `--version` flag to display the current version
  ([#22](https://github.com/raine/claude-history/issues/22))
- Invalid session IDs now show a clear error message instead of failing silently

## v0.1.38 (2026-03-13)

- Improved search for CJK (Chinese, Japanese, Korean) text — queries with CJK
  characters now match correctly even when words aren't separated by spaces
  ([#19](https://github.com/raine/claude-history/pull/19))
- Added emacs-style keybindings to the search input: `Ctrl+A`/`Ctrl+E` to jump
  to start/end, `Ctrl+B`/`Ctrl+F` to move by character, `Alt+B`/`Alt+F` and
  `Ctrl+Left`/`Ctrl+Right` to move by word, `Ctrl+K` to kill to end of line,
  `Ctrl+U` to kill to start of line
  ([#19](https://github.com/raine/claude-history/pull/19))
- Fixed cursor alignment issues with wide characters (e.g. CJK, emoji) in the
  search input and conversation viewer
  ([#19](https://github.com/raine/claude-history/pull/19))

## v0.1.37 (2026-03-13)

- Linux prebuilt binaries are now statically linked, fixing compatibility issues
  on older distros with outdated glibc versions
  ([#20](https://github.com/raine/claude-history/issues/20))

## v0.1.36 (2026-03-12)

- Added message-level navigation — press `J`/`K` or `[`/`]` to jump between
  messages in the conversation viewer, with a teal marker showing the focused
  message ([#15](https://github.com/raine/claude-history/pull/15))
- Added single message copy — press `y` to copy the currently selected message
  to the clipboard instead
  ([#15](https://github.com/raine/claude-history/pull/15))
- Fixed empty thinking blocks rendering as blank "Thinking" labels with no
  content

## v0.1.35 (2026-03-12)

- Timestamps in the conversation list now automatically switch between relative
  ("just now", "5 min ago", "2 hours ago", "yesterday") for recent sessions and
  absolute ("Mar 05, 14:30") for older ones
- Recent conversations are color-graded by recency — newest sessions appear in
  bright teal, fading to gray as they get older, making it easy to spot recent
  activity at a glance
- Removed `--relative-time`/`--absolute-time` flags and `display.relative_time`
  config option — the new hybrid format replaces both

## v0.1.34 (2026-03-12)

- Search now covers tool output (command results, file contents, grep output),
  so you can find conversations by content that previously only appeared in tool
  calls
- Search highlighting now merges adjacent matches across separators — searching
  "red team" highlights the full word `red_team` instead of just the individual
  parts
- Improved search performance in conversations with large tool outputs

## v0.1.33 (2026-03-12)

- Added automatic light/dark theme detection — the TUI now adapts its color
  scheme to match your terminal background
- Fixed arrow key navigation lag when holding keys to scroll quickly through the
  list or conversation viewer
- Fixed slow redraw when pasting text into the search field

## v0.1.32 (2026-03-12)

- Fixed clipboard copy/yank not working on Linux — now uses `wl-copy` on Wayland
  and `xclip`/`xsel` on X11, with automatic display server detection
  ([#17](https://github.com/raine/claude-history/pull/17))
- Fixed resuming sessions from deleted or ephemeral git worktrees failing with
  an error instead of gracefully recovering

## v0.1.31 (2026-03-09)

- Search now matches project names, so you can find sessions by the project they
  belong to

## v0.1.30 (2026-03-09)

- Preview panel now shows the last messages by default instead of the first, so
  you see the most recent context at a glance (use `--first` to revert)

## v0.1.29 (2026-03-09)

- Added `--fork-session` flag to resume a conversation as a fork, creating a new
  branch from an existing session
- Cross-project forking: when forking a session from a different project, the
  session files are automatically copied to the current project so Claude
  resumes in the right context
- Added configurable keybindings via the `[keys]` section in the config file,
  allowing rebinding of resume (`Ctrl+R`), fork (`Ctrl+F`), and delete
  (`Ctrl+X`) actions
- Session list search now matches session UUIDs, making it easier to find a
  specific conversation by ID
- Fixed markdown rendering issues: soft breaks no longer collapse words
  together, inline code no longer clips at block edges, and list item spacing is
  correct

## v0.1.28 (2026-03-04)

- Subagent (Task tool) messages are now nested under their parent task, keeping
  the conversation view clean and organized with `↳` prefixed entries
- Subagent internals are hidden by default and revealed with `T` or
  `--show-thinking`, same as thinking blocks
- XML-tagged content (system reminders, analysis blocks) now displays correctly
  instead of being silently stripped
- Conversations from CI or headless Claude runs that lack timestamps now parse
  and display correctly

## v0.1.27 (2026-02-26)

- Session titles (set via `/rename` in Claude Code) now appear in the
  conversation list and viewer, making it easier to find named sessions
- Search preview shows matches better now

## v0.1.26 (2026-02-18)

- Added `global = true` config option to default to global search without
  passing `-g` every time, with `--local` flag to override when needed
- Ledger export and clipboard copy now render markdown properly (headings,
  lists, code blocks, tables) and wrap long lines instead of overflowing
- Fixed high idle CPU usage (~9% down to near zero) when the TUI was sitting
  idle after loading
- Fixed search preview highlighting partial word matches instead of the actual
  search phrase
- Fixed long lines in code blocks overflowing the terminal width
- Fixed blank lines and indentation issues in ledger export

## v0.1.25 (2026-02-11)

- Added `--show-id` (`-i`) flag to print the selected conversation's session ID,
  useful for resuming with custom shell aliases (e.g.,
  `claude --resume $(claude-history --show-id)`)
- Added `I` keybinding in the viewer to copy the session ID to clipboard

## v0.1.24 (2026-02-11)

- Tool calls now default to **truncated** mode, showing the header and first few
  lines with a "(N more lines...)" indicator — a middle ground between hidden
  and full output. Press `t` to cycle through modes: off, truncated, full
- Added `--no-tools` flag to start with tools hidden (complements `--show-tools`
  for full mode)
- Tables in conversation output are now rendered with proper box-drawing borders
  instead of being collapsed into plain text

## v0.1.23 (2026-02-08)

- Fixed blank or empty message blocks occasionally appearing in conversation
  output

## v0.1.22 (2026-02-07)

- Added multi-word search support in the viewer — search for phrases like "add
  feature" to find matches containing both words
- Timestamps now display on tool calls and results in ledger view (when timing
  is enabled with `i`)
- Fixed a crash that could occur when highlighting search matches containing
  certain Unicode characters

## v0.1.21 (2026-02-05)

- Fixed timestamp alignment for subagent messages and empty messages in ledger
  view
- Fixed double blank lines appearing after tool calls with empty output
- `/clear` commands are no longer shown in conversation rendering

## v0.1.20 (2026-02-05)

- Added toggleable timing display in conversation viewer — press `i` to show
  timestamps next to each message
- Show conversation duration and model/token count in the viewer header
- Show conversation duration in the conversation list
- Added keyboard shortcuts help overlay — press `?` in any view
- Added keyboard shortcuts bar at the bottom of the conversation list
- Added `Ctrl+R` (resume) and `Ctrl+X` (delete) shortcuts to the viewer status
  bar
- Added `Ctrl+C` to quit from viewer mode
- Exports now include thinking blocks and tool calls when their display is
  toggled on
- Long bash commands in tool calls are now wrapped for readability
- Improved search highlight color for better visibility

## v0.1.19 (2026-02-04)

- Added syntax highlighting for code blocks in conversation output
- Improved tool call display with human-readable formatting instead of raw JSON
- Added Vim-style half-page navigation (Ctrl-D/Ctrl-U) in the viewer
- Added Ctrl-W to delete word before cursor in the search field
- Show conversation summary in the viewer header and search results
- Display subagent conversations in ledger view
- Added direct JSONL file input support (pass a file path as argument)
- Added `--render` flag for debugging ledger output
- Improved header layout: combined into single line when terminal width allows
- Tool/thinking toggle settings now persist within session

## v0.1.18 (2026-02-02)

- Added in-TUI conversation viewer. Press Enter to view conversations without
  leaving the TUI, with Vim-style navigation (j/k, d/u, g/G) and search (/)
- Added export and yank menus to the viewer. Press `e` to export to file or `y`
  to copy to clipboard in multiple formats (ledger, plain text, markdown, JSONL)
- Added `Y` hotkey to copy the conversation file path to clipboard
- Added `resume.default_args` config option to pass custom arguments when
  resuming conversations with `Ctrl+R`
- Improved markdown rendering: fixed spacing after numbered lists, styled
  headings with subtle color
- Fixed thinking blocks to render with italic and dimmed style
- Fixed user messages showing in wrong color in the viewer
- Improved search performance

## v0.1.17 (2026-02-01)

- Added `Ctrl+R` keybinding to resume the selected conversation directly from
  the TUI

## v0.1.16 (2026-02-01)

- Fixed a crash when using global search (`-g`) that could occur when deleting
  conversations

## v0.1.15 (2026-02-01)

- Added ability to delete conversations from the TUI (press `Ctrl+D`, confirm
  with `y`)
- Added cursor navigation in the search field with arrow keys

## v0.1.14 (2026-02-01)

- Added markdown rendering for conversation output with support for headings,
  lists, code blocks, tables, and inline formatting
- Added pager support—long conversations now open in `less` (or `$PAGER`)
- Added `--plain` flag for unformatted output
- Improved search to better match word variations (e.g., "config" now matches
  "configuration")
- Added `curl | bash` install script
- Hide caveat metadata from conversation previews

## v0.1.13 (2026-02-01)

- Replaced fzf with a built-in terminal UI

## v0.1.12 (2026-01-11)

- Fixed project path detection failing for usernames containing dots (e.g.,
  `my.user`) (Thanks @duke8585!)

## v0.1.11 (2025-12-20)

- Cleaned up fzf picker display by removing index numbers

## v0.1.10 (2025-12-15)

- Added a specific error message when fzf version is too old (requires 0.67.0+)

## v0.1.9 (2025-12-14)

- Added color highlighting to the fzf picker

## v0.1.8 (2025-12-14)

- Improved fzf UX: the timestamp stays visible when searching

## v0.1.7 (2025-12-14)

- Added `--global` (`-g`) flag to search conversations across all projects at
  once

## v0.1.6 (2025-11-29)

- Added `--all-projects` (`-a`) flag to browse conversations from any project
- Added `--show-path` (`-p`) flag to print the selected conversation's file path
- Improved fuzzy search to match against full conversation content
- Added Homebrew installation support

## v0.1.5 (2025-11-17)

- Added display of tool call inputs and results when viewing conversations
- Fixed project detection for paths containing dots or special characters

## v0.1.4 (2025-10-30)

- Added faster startup with parallel conversation loading

## v0.1.3 (2025-10-30)

- Added `--debug` flag to show diagnostic information about conversation loading
- Fixed conversations containing only `/clear` commands incorrectly appearing in
  the list
- Cleaned up `/clear` command metadata from conversation previews
- Used file modification time for more accurate conversation dates

## v0.1.2 (2025-10-29)

- Fixed display of tool results that contain structured content instead of plain
  text

## v0.1.1 (2025-10-29)

- Added configuration file support (`~/.config/claude-history/config.toml`) for
  persistent display preferences
- Added `--show-thinking` and `--hide-thinking` flags to control visibility of
  Claude's thinking blocks
- Hidden tool calls by default (use `--show-tools` or `-t` to show them)
- Added `--first` flag to show first messages in preview (inverse of `--last`)
- Added `--absolute-time` flag to explicitly use timestamps (inverse of
  `--relative-time`)
- Fixed message preview order when using `--last` flag

## v0.1.0 (2025-10-29)

- Initial release
