# Move `sisho tui` out as the extension `sisho-desk`

Status: not started. Written 2026-10-20 from a design talk with the author. Read `tmtroot/dev/design.tmt` and `AGENTS.md` first; this file assumes them.

## Why

`sisho` is meant to be extended by programs named `sisho-<word>` on `PATH`, run as `sisho <word>` (the cargo way). An extension mechanism nobody has built an extension for is a guess. So the first extension is ours, and it is the interactive tool we have today.

What `sisho tui` does now (Explorer, Migration, BatchMeta, StructuralGrep) edits Tomet documents. It is what someone who has just started with Tomet wants first, but it is not what the orchestrator itself is about, and the community may well write better explorers. So:

- The current TUI leaves the `sisho` binary and becomes the extension **`sisho-desk`**, a new repository. Run as `sisho desk`. It is the reference extension, and the batteries that come first.
- `sisho` gets a TUI of its own later, for operating `sisho` itself (steps running, tool versions, broken links, installed extensions), not for editing documents. Not part of this task (see "Not in this task").

## Decisions already made (do not reopen; ask the author if one is in your way)

1. The extension is `sisho-desk`, the repository is `sisho-desk`, the word is `desk`. (`workbench` was considered and dropped: a beginner should not have to type it. `desk` goes with `shelf`.)
2. `sisho tui` goes away. **No compatibility alias**, one move, as with the move from tomet. The `tui` word is free afterwards.
3. Built-in commands and the programs `sisho` already knows (`tomet`, `twrit`, `tandr`, `shelf`) always win over an extension of the same word.
4. An extension is a separate process. `sisho` passes the rest of the command line unchanged, hands over the terminal, and takes the extension's exit code, exactly as `sisho shelf` does. `sisho` does not ask an extension for a version and holds it to no minimum (same as `shelf`).
5. **`sisho-desk` depends on no `sisho` crate.** The only place the old code used one is `sisho_run::Step` for `$EDITOR` (`crates/sisho-tui/src/lib.rs:535`); it becomes `std::process::Command`. The extension knows the contract, not the orchestrator.
6. The code moves **unchanged in behaviour**: all four tabs stay as they are, StructuralGrep included (tandr will take it over later). Trimming tabs is a separate decision of the author. The only edits are the renaming from "workbench" to "desk" in help text and docs, and the `sisho_run` removal.
7. `sisho-desk` keeps pinning the `tomet-*` crates itself, through git dependencies and its own `Cargo.lock`, as `twrit` does. This is not the diamond the memo warns about: the two programs never share a type, since they are separate processes.

## Open questions for the author (bring them back; do not decide)

- Does the `toolchain` package of the sisho flake include `sisho-desk`? The memo defines `toolchain` as the core three (`sisho`, `tomet`, `twrit`). Recommendation: expose `sisho-desk` as its own package and in `combine`, leave `toolchain` alone until the author says.
- What should `sisho desk` say when `sisho-desk` is not installed? With the generic lookup it is the same as any unknown word (exit 2, "neither a command nor a tool"). A beginner typing `sisho desk` deserves a clearer line. Recommendation: the generic message gets one added sentence ("an extension is a program named `sisho-<word>` on PATH"), and nothing about `desk` specifically is hard-coded in `sisho`.
- The sentence in `AGENTS.md` / `tmtroot/agents.tmt`, "`sisho` is not a task runner. It does not execute arbitrary commands", needs wording that allows `sisho-<word>` lookup and nothing else. Propose the wording; the author approves it.

## Rules of the repository that touch this task

- Never edit `.md` files that are generated; edit the `.tmt` under `tmtroot/` and run `just docs`, then `just docs-check`.
- **`.writ.tmt` is edited by the author only.** Phase 4 needs edits to it (listed there). Write the proposed text in the task file or the final report and stop; do not edit it and do not loosen a rule to fit.
- Code comments and documentation in English. The design memo `tmtroot/dev/design.tmt` is the one document written in Japanese; keep its style when you add to it.
- Commits: `type(scope): description`; no AI trailers; ask the author before committing; never push unasked. The repository uses `jj`; use `--no-pager`.
- Nix: never `nix develop path:.` (it copies `target/` into the store; it filled a disk on 2026-10-18). Stage new files with `git add -N .` and use plain `nix develop`. `/tmp` is a small disk; build directories do not go in the scratchpad.
- Work in a dedicated workspace under `.agents/workspaces/` (see `AGENTS.md`), and keep this file up to date after each step.

## Phase 1: `sisho <word>` finds `sisho-<word>` (in this repository)

This must land before Phase 4, or `sisho desk` has nothing to run.

- [x] In `app/src/commands/tool.rs`: when `program(word)` is `None`, run `sisho-<word>` the way a program is run now (`Step::new(..).args(..).run_inherited()`), with the same exit-code rules (`passthrough_code`).
  - If the program is not found, keep today's behaviour for an unknown word: message on stderr, exit 2 (usage error). Do not exit 127 here; 127 stays for a *known* program that is not installed (`sisho tomet` without `tomet`).
  - Accept only words made of lowercase letters, digits and `-`, not starting with `-` (so `sisho ../x` or `sisho 'a/b'` is a usage error, never a path to run). Test it.
  - Do not scan `PATH`. `sisho-tools` is under the `tools-by-subprocess` guard (no `std::fs`); a plain run is enough and needs none.
- [x] Keep `sisho-tools::program()` as the table of known programs. The extension fallback lives in `app`, so the table stays a hand-written list.
- [x] Tests in `app/tests/tool.rs` with fake extensions on a `PATH` the test controls (see `app/tests/common`): arguments and flags unchanged; exit code kept; terminal/stdin handed over; unknown word and nothing on `PATH` exits 2; a built-in or known program wins over a `sisho-<same word>` on `PATH` (for example a fake `sisho-check` and `sisho-tomet` must never run); invalid words exit 2.
- [x] Help: the `Tool` command's doc comment and the error message in `tool.rs` say that `sisho-<word>` on `PATH` is also run.
- [x] Docs: a spec `tmtroot/spec/cli/extension.tmt` (`@meta{ id: cli.extension }`, English; look at `tmtroot/spec/cli/*.tmt` for the style) that states the contract in decisions 3 and 4. A short section in the design memo recording decisions 1 to 7 with the date 2026-10-20 and the reason (a reference extension is what proves the mechanism). Update `tmtroot/readme.tmt` (Usage). Run `just docs`, `just docs-check`.
- [x] `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `twrit check .` clean.

## Phase 2: the new repository `sisho-desk`

Lay it out like its siblings (`../twrit`, `../tomet-shelf`): Cargo workspace, `app/` (package `sisho-desk`, binary `sisho-desk`), one library crate for the moved code, `flake.nix` + `nix/`, `justfile` (`build`, `test`, `docs`, `docs-check`), `tmtroot/{readme,agents,contributing}.tmt` with the generated `.md` files, `default.config.tmt`, `.writ.tmt`, dual license (MIT OR Apache-2.0), `.editorconfig`, `.gitattributes`, `.gitignore`, `.envrc`, jj colocated. `CLAUDE.md` is the one line `@AGENTS.md`.

- [ ] Copy `crates/sisho-tui/src/**` and its tests. Keep the module structure (`app/`, `engine/`, `ui/`).
- [ ] Replace `sisho_run::Step` at `lib.rs:535` with `std::process::Command`. The behaviour must stay: the editor is `EDITOR`, then `VISUAL`, then `nano`; the terminal is given back before it starts and the workspace is read again when it ends; a failure to start is shown, not a crash.
- [ ] Dependencies: the `tomet-*` crates the code uses (`tomet-ast`, `-config`, `-indexer`, `-load-config`, `-markdown`, `-parser`, `-printer`, `-semantics`, `-transform`, `-tree`, `-workspace`, `tree-sitter-tomet`), `ratatui` 0.29, `crossterm` 0.28, `tree-sitter` 0.26, `anyhow`, and `uuid` for tests. Copy the exact git URL and the pinned revision from `sisho`'s `Cargo.toml` and `Cargo.lock` so the first build is the same code.
- [ ] Command line: `sisho-desk [PATH] [-c, --config FILE]`, `--help`, `--version` (a plain `clap` binary). `PATH` defaults to `.`. Same behaviour as `sisho tui`: no terminal on standard input means a message on stderr starting `sisho-desk: `, exit 1, nothing drawn; the terminal is always given back (normal quit, error while reading, panic).
- [ ] Exit codes as in the rest of the ecosystem: 0 on a normal quit, 1 when it could not do what was asked, 2 for a usage error.
- [ ] Rename "workbench" to "desk" in every help text, doc comment and doc. Move `tmtroot/spec/cli/tui.tmt` of `sisho` here, rewritten for `sisho-desk` (`@meta{ id: ... }` chosen to match the new repository's spec naming; English). It must say that the program is also reached as `sisho desk`.
- [ ] A rule in the new `.writ.tmt` (a proposal to the author, who edits that file) that no `sisho-*` crate is a dependency, so decision 5 is checked and not remembered.
- [ ] Tests: carry the 7 moved unit tests and the 7 `TestBackend` render tests; the 2 integration tests of `app/tests/tui.rs` become tests of the new binary (`CARGO_BIN_EXE_sisho-desk`), with the help check changed from "workbench" to "desk".
- [ ] **Parity check.** Before anything is removed from `sisho`, show that `sisho-desk` is the same program: on one vault, in a tmux virtual terminal, compare `sisho tui` and `sisho-desk` on 10 screens (the start screen and 9 key presses covering each tab, `/` in Explorer, selecting and the confirmation in Migration, `k` / `v` in BatchMeta, the conditions in StructuralGrep, and quitting with `q`). Identical screens, identical exit codes. Keep the script in the new repository's `scripts/` so it can be run again. (The same check was done for the move from tomet on 2026-10-18; its script is gone.)
- [ ] `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `just docs-check`, `twrit check .` clean.

## Phase 3: Nix

- [ ] `sisho-desk`'s own flake builds the package, in the same shape as `../tomet-shelf` (crane, fenix, flake-parts, treefmt-nix). Its tool inputs are cut with `follows = ""` for anything that only serves another tool's development, as the seven flakes switched on 2026-10-18 do (design memo, "Nix で束ねる"). It must not take `sisho` as an input.
- [ ] `sisho`'s `flake.nix`: add `sisho-desk` as an input (cut the same way) and as `packages.sisho-desk`, and make it available in `legacyPackages.<system>.combine`. Do not change `toolchain` (open question above). Measure the `flake.lock` node count before and after and record it in the design memo; it should stay near 32 + the new input's own nodes. Remember that a `follows = ""` only takes effect once the locked revision has the input name.
- [ ] `nix build` of both packages passes its tests; `sisho desk` works from a dev shell that has both on `PATH`.

## Phase 4: remove `tui` from `sisho`

Only after Phases 1 to 3, and after the author has seen the parity check.

- [ ] Delete `crates/sisho-tui`, `app/src/commands/tui.rs`, `app/tests/tui.rs`, the `Tui` variant and `TuiArgs` in `app/src/cli.rs`, its dispatch in `app/src/main.rs`, and `sisho-tui` from the workspace `Cargo.toml` and `app/Cargo.toml`.
- [ ] Remove the workspace dependencies that nothing else uses any more, then check each against the remaining crates: `ratatui`, `crossterm`, `tree-sitter`, `tree-sitter-tomet`, `tomet-transform`, `tomet-tree`, `tomet-config`, `uuid`. (`sisho-vault` still uses `tomet-indexer`, `-markdown`, `-parser`, `-printer`, `-semantics`; verify the rest with `cargo tree`.) Update `Cargo.lock`.
- [ ] Remove `tmtroot/spec/cli/tui.tmt` (it lives in `sisho-desk` now), and the `tui` line of the Usage and the `crates/sisho-tui` line of Workspace Layout in `tmtroot/readme.tmt`. Add `sisho desk` to Usage as the first extension. Regenerate with `just docs`.
- [ ] `tmtroot/spec/cli/extension.tmt` lists `tui` among the built-in commands; remove it from that list when `sisho tui` is removed.
- [ ] Design memo: a dated entry (2026-10-20 or the day it is done) saying where the TUI went and why; fix the older line about `tui` ("tui を sisho に移す") so it does not read as the final state.
- [ ] Other repositories that name the old command: `../tomet/tmtroot/spec/cli/overview.tmt:77` says `sisho tui` takes `--config`; it becomes `sisho desk`. Search `../*` again before you finish (`grep -rIn -E 'sisho tui|tomet tui|sisho-tui'`, excluding `target/` and `tmtroot/deprecated/`); `../tomet/editors/zed/grammars/typedmark` is old and is left alone.
- [ ] **Proposed edit to `.writ.tmt` (the author applies it):** remove `crates/sisho-tui` from layer 1 of `crate-layering`; in the prose of that rule, `sisho-serve` is the only crate kept apart for its stack and "terminal-UI" goes; in `tools-by-subprocess`, the list of crates that use the `tomet` crates loses `sisho-tui`. Put the exact diff in your report. Until the author applies it, `twrit check .` will complain about a missing workspace member; say so rather than working around it.
- [ ] `cargo test --workspace`, clippy, `just docs-check`, `twrit check .` (except for the `.writ.tmt` item above).
- [ ] Move what is worth keeping into the memo and the specs, then delete this task file (as `AGENTS.md` asks).

## Not in this task

- The built-in TUI of `sisho` itself, shown when `sisho` is run with no arguments (today that prints help: `arg_required_else_help = true`). It is to be designed with the author first. Likely contents, none of them decided: steps of `check` / `fix` running with their outcomes, tool versions against the minimums, the extensions found, broken links from `check-links`. It should stay out of the core build behind a cargo feature if it needs `ratatui`.
- Trimming or reshaping the tabs of `sisho-desk` (Explorer, StructuralGrep to be taken over by tandr).
- A way for `sisho` to list the extensions installed (it would need to scan `PATH`, which `sisho-tools` may not do under the `pure` guard, so it would live in `app`).
- Having `sisho-desk` read tool output through `--json` instead of the `tomet-*` crates.
