# Work order: sisho counter tabbed views (`[1: Health]` / `[2: Repos]`)

## Objective
Introduce two tabbed views in `sisho`'s default interactive counter TUI:
- `[1: Health]`: The existing project integrity and health dashboard (Tools & Extensions, Broken Links, Checks, Output Preview).
- `[2: Repos]`: The multi-repo / workspace dashboard (Discovered repositories via `catasto`, sync/drift status, per-repo details, and sync/update/diff action triggers).

## Philosophy & Architecture Rules
- Sisho calls core tools (`tomet`, `catasto`, `twrit`) as subprocesses only; never links them as crates.
- User-first design: Clean tab switching (`1`, `2`, `Tab`), responsive UI, intuitive action shortcuts.
- No hacky shortcuts or legacy excuses.
- English code comments and docstrings.
- Full verification: `just build`, `just test`, `just clippy`, `just docs-check`.

## Discrete Steps
- [x] 1. Model & Tab Architecture (`crates/sisho-counter/src/model.rs`):
  - Add active tab enum: `ActiveTab::Health`, `ActiveTab::Repos`.
  - Add repository state to `Model`: discovered workspaces, scan status, selected repository index, per-repo outcome (violations, status, clean/drift).
  - Add background runner to invoke `catasto check --all --json` (or `catasto check --json` if single repo) to populate repository status.
  - Implement actions for Repos tab: sync selected (`catasto sync <dir>`), sync all (`catasto sync --all`), update (`--update`), diff (`catasto diff`).
- [x] 2. UI Layout & Rendering (`crates/sisho-counter/src/ui.rs`):
  - Top tab bar: `[1: Health]  [2: Repos]`.
  - In `ActiveTab::Health`: Render existing 4 panes.
  - In `ActiveTab::Repos`:
    - Left column: List of discovered workspaces with status icons (`✓ clean`, `✗ drift`, `! error`).
    - Right column: Details of selected repository (managed files, violations, diff/error preview).
  - Bottom Counter bar: Contextual actions for active tab (e.g. `[s] Sync  [S] Sync All  [u] Update  [U] Update All  [1/2] Tab`).
- [x] 3. Key Event Handling (`crates/sisho-counter/src/lib.rs`):
  - Handle `1`, `2`, `Tab`, `BackTab` for tab switching.
  - In Repos tab, handle `j`/`k`/Up/Down navigation and action keys (`s`, `S`, `u`, `U`, `d`, `r`).
- [x] 4. Tests (`crates/sisho-counter/src/model_tests.rs`, `render_tests.rs`):
  - Unit tests for tab switching, repo navigation, and key handling.
  - Render tests for both tabs.
- [x] 5. Verification:
  - Run `just build`, `just test`, `cargo clippy --workspace --all-targets` in `sisho`.
  - All tests pass (30 in sisho-counter, 100+ across workspace).
  - Clippy reports 0 warnings across all targets.
