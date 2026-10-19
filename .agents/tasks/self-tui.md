# A TUI for `sisho` itself, shown when `sisho` is run with no arguments

Status: not started. **Design first; do not write code before the author has agreed on the design.** Recorded 2026-10-20, at the end of the move of `sisho tui` out to `sisho-desk` (see `tmtroot/dev/design.tmt`, entries of 2026-10-20).

## What this is, and what it is not

Today `sisho` with no arguments prints help (`arg_required_else_help = true` in `app/src/cli.rs`). The author decided (2026-10-20) that the entry of `sisho`'s own interactive screen is **bare `sisho`**, not a `tui` word. The word `tui` is simply free.

This TUI operates **`sisho` itself**: what it runs, what it found, what is broken. It does not edit Tomet documents. Editing and migrating documents is `sisho-desk` (`sisho desk`), the reference extension. Do not rebuild any of its tabs here.

## Decided

1. Entry: bare `sisho`. `sisho --help`, `-h` and `--version` are unchanged.
2. Scope: operating `sisho` itself. Document editing stays in `sisho-desk`.
3. It stays out of the core build if it needs a TUI stack: behind a cargo feature, the way `sisho-serve` keeps the HTTP stack apart. A plain `cargo build` of `sisho` for people who never open the screen must not pull `ratatui` unless the author says otherwise.
4. The rules of the memo still hold: no manifest of its own, not a task runner (it starts only the tools it knows by name and `sisho-<word>`, `AGENTS.md`), nothing in a vault, a config or a manifest names a program to run.

## Open (the design step settles these with the author)

Candidate contents, none chosen:

- The steps of `check` / `fix` running, with their outcome as it arrives (the `Outcome` / `Status` types in `sisho-run`, the job server of `-j`), and a way to run them again.
- The tools found, their versions against the minimums (`sisho-tools`: `Version`, `judge`, `Tool::min_version`, 0.2.0 today), and what to do when one is old or missing.
- Broken links from `check-links`, with the place of each.
- The installed extensions. This needs the extension list (`extension-list.md`, same directory); do that task first or together.
- A way to hand a file to `sisho desk` to edit it. There is no contract for that today (`sisho-desk [PATH]` opens a directory or file and nothing more). Do not invent one here; ask.

Questions to answer in the design:

- What does bare `sisho` do when standard input or output is not a terminal (a script, a pipe)? Recommendation: print help as today, so no script that calls bare `sisho` changes. The author decides.
- Does the screen run things (`check`, `fix`) or only show what ran? `fix` writes files; if it is offered, it asks first, as `sisho-desk` does for every write.
- Where does the code live? A new crate means a new layer in `.writ.tmt` (`crate-layering`) and a decision on `process-door` (only `sisho-run` starts a process, so the screen runs steps through it). `.writ.tmt` is edited by the author only: propose the text, do not edit.
- The terminal must be given back however the program ends (a normal quit, an error, a panic). `sisho-desk` has a guard for this (`Screen` and a panic hook in `crates/sisho-desk-core/src/lib.rs`); `sisho` may not depend on it (the dependency runs from sisho to the tools, never the other way and never to an extension). Copy the idea, not the crate, and say so in the design.

## Steps

- [ ] Design memo section in `tmtroot/dev/design.tmt` (Japanese, like the rest): purpose, the screens with sketches, what each shows and where its data comes from, what it may start, behaviour without a terminal, the crate and layer. Show it to the author. **Stop here until it is agreed.**
- [ ] Spec `tmtroot/spec/cli/` for the screen (English, `@meta{ id: cli.... }`): starting, keys, what is written when (nothing, unless `fix` is offered and confirmed).
- [ ] Implementation behind the cargo feature; terminal guard; no terminal means help.
- [ ] Tests: `TestBackend` render tests for each screen; an integration test for "no terminal"; the same tmux check as for the move of `tui` (start screen and a series of key presses on a virtual terminal), kept as a script in `scripts/` only if it can be run again.
- [ ] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `just docs-check`, `twrit check .`.
- [ ] Update `tmtroot/readme.tmt` (Usage) and run `just docs`.

## Not in this task

Editing documents; a plugin protocol for panes from extensions (the contract with extensions stays: exit code, `--json`, `--version`, a terminal handed over); anything for `tandr` before it has a command line.
