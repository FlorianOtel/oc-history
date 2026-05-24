---
title: "oc-history — Changelog"
created_at: 2026-05-24--09-45
created_by: Florian Otel florian.otel@gmail.com
updated_by: Claude Code (Claude Sonnet 4.6)
updated_at: 2026-05-24--20-33
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
