# Task: Broken Links Editor Handover

## Goals
- Allow opening a broken link's source file at its line number from `sisho-counter` using an editor.
- Respect the editor precedence chain: `$SISHO_EDITOR` -> `$VISUAL` / `$EDITOR` -> `sisho-desk` -> `nano`/`vi`.
- Support `{file}` and `{line}` templates in `$SISHO_EDITOR`, as well as standard `+<line> <file>` for Unix editors.
- Seamless terminal handover with RAII screen restoration.
- Trigger automatic `check-links` re-verification upon returning from the editor.

## Discrete Steps
- [x] 1. Create task tracking file and Jujutsu workspace `.agents/workspaces/broken-links-editor`.
- [ ] 2. Implement `crates/sisho-counter/src/editor.rs` with editor resolution and `Step` construction.
- [ ] 3. Add unit tests for editor resolution (templates, standard Unix editors, fallbacks).
- [ ] 4. Connect editor launching to `Enter` and `e` in `lib.rs`, and update `ui.rs` key prompts.
- [ ] 5. Update render tests in `sisho-counter`.
- [ ] 6. Update `tmtroot/spec/cli/counter.tmt` and run `just docs` / `just docs-check`.
- [ ] 7. Run `cargo test` across workspace.
- [ ] 8. Clean up workspace and review changes with user.
