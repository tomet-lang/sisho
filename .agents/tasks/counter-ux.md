# Task: Sisho Counter UX Enhancements

## Goals
1. **Real-time per-step check progress**:
   - Stream check step outcomes as they complete instead of a single "Running checks..." line.
   - Display each step (tomet check, format, export, twrit) with live running/pending spinner or finished status icon and duration.
2. **Broken Links pane interaction & scrolling**:
   - Support pane switching with `Tab` / `BackTab`.
   - In Broken Links pane, enable `j`/`k`/`Up`/`Down` selection and scrolling for list of broken links.
   - Highlight active pane border/title.
3. **Step Output Preview scrolling**:
   - Remove hardcoded `.take(15)` limitation in output preview.
   - Allow vertical scrolling (`j`/`k`/`Up`/`Down`/`PageUp`/`PageDown`) when Preview pane is focused.
   - Show scroll position indicator in title or status.
   - Reset scroll when selecting a different check step.

## Note
- Do not modify or alter the 4-row layout height of the Integrity box ("4行化はもうやった。変なことしないで。").

## Discrete Steps
- [x] 1. Create Jujutsu workspace `.agents/workspaces/counter-ux` on top of `@`.
- [x] 2. Update `sisho-run` with `run_parallel_streaming` to report step completion in real time.
- [ ] 3. Update `sisho-counter` `Model` to:
       - Track per-step statuses for checks (pending, running, done).
       - Stream background check & fix outcomes via channel.
       - Track `focused_pane` (`Checks`, `BrokenLinks`, `OutputPreview`).
       - Track `selected_broken_link` and `preview_scroll`.
- [ ] 4. Update `sisho-counter` `ui.rs` to:
       - Render live per-step spinner/status in Integrity pane while keeping the 4-row layout.
       - Render Broken Links with cursor and scroll window based on active selection.
       - Render Output Preview with scrolling and remove `.take(15)`.
       - Visually distinguish the focused pane.
- [ ] 5. Update `sisho-counter` `lib.rs` event loop to handle `Tab`, `BackTab`, and pane-specific scrolling.
- [ ] 6. Update and add unit/render tests in `crates/sisho-counter` and `crates/sisho-run`.
- [ ] 7. Run `cargo test` and `just docs-check` to verify all tests and rules pass.
- [ ] 8. Clean up workspace and review changes with user.
