# Scaffold the repo, then write the implementation plan

Design memo: `tmtroot/dev/design.tmt` (agreed 2026-10-18).

## Scaffold (done 2026-10-18)

- [x] Cargo workspace with `app/` (`sisho-cli`, bin `sisho`); `sisho --version` works, no subcommands yet.
- [x] `tmtroot/{readme,agents,contributing}.tmt` -> generated `README.md`, `AGENTS.md`, `CONTRIBUTING.md`. `CLAUDE.md` is the single line `@AGENTS.md`.
- [x] `flake.nix` + `nix/` (inputs: nixpkgs, flake-parts, treefmt-nix, fenix, crane, tomet). `flake.lock` and `Cargo.lock` generated.
- [x] `justfile` (`docs`, `docs-check`), `default.config.tmt`, `.editorconfig`, `.gitattributes`, `.gitignore`, `.envrc`, dual license.
- [x] jj colocated repo initialised. Nothing committed yet.
- [x] No `.github`, no `.writ.tmt` (see plan item 1).

## Findings during scaffolding

- `tomet`'s flake takes `twrit` as an input, so locking `sisho` pulled `twrit` in transitively; `nix flake lock` resolved without looping. Whether sisho should pin tomet and twrit explicitly is still open (design memo, "Undecided").
- Nix flakes only see git-tracked files. In a colocated jj repo the new files are not in the git index until committed, so before the first commit use `nix develop path:.` instead of `nix develop`.

## Next: implementation plan

1. [x] Layer structure of the crates, as stub crates plus `.writ.tmt` (done 2026-10-18):
   layer 2 `app`; layer 1 `sisho-tools`, `sisho-vault`, `sisho-serve`, `sisho-tui`; layer 0 `sisho-run`.
   Rules: `crate-layering`, `tools-by-subprocess` (`pure`, empty `allow-external`), `process-door` (`std::process` only in `sisho-run`), `tmtroot-output-not-edited`.
   Verified with `twrit check .`, and each guard was seen to fail in a scratch copy (upward edge, `std::process` in `sisho-tools`, external crate in `sisho-run`).
   Not yet committed. Notes: an empty list must be spelled `list()` (`[]` is rejected by twrit); `tmtroot/schema/vocabularies/writ.vocabulary.tmt` is a copy of twrit's, referenced from `default.config.tmt`; `twrit` is not in the dev shell, so run it from `../twrit/target/debug/twrit`.
2. Subprocess runner and the `check` / `fix` verbs for tomet and twrit. Design agreed 2026-10-18 (see "check / fix の実行モデル" in the design memo):
   - [x] `--version` added to `tomet` and `twrit` (one line each in their `cli.rs`; builds, `cli-docs --check` still clean; uncommitted in those repos).
   - [x] `sisho-run` (done 2026-10-18): `Step` (run with captured output, stdin closed), `Outcome` / `Status` (passed / failed(code) / signalled / not found / could not start), `Job` + `run_parallel` (plan order, one thread per job, steps and in-process work in one list), `Report` (exit code 0 / 1). 12 unit tests using `sh`; clippy and the three writ guards clean. The fifth status `CouldNotStart` was added beyond the four in the memo (a program that exists but cannot be executed).
   - [x] `sisho-tools` (done 2026-10-18): `Tool` (`Tomet`, `Twrit`; `tandr` joins when it has a CLI), `check_steps(path)` = `tomet check`, `tomet format --check`, `twrit check`; `fix_steps(path)` = `twrit check --fix`, `tomet format -i`. `Step` in `sisho-run` now holds `OsString`s so a non-UTF-8 path survives. 8 unit tests, including "no write flag in a check step" and "every tool has a check step".
   - Left out on purpose until they move into `sisho-vault`: `export --check` / `export`, `check-links`, `refactor`. They join the plans as in-process steps.
   - [x] `app` (done 2026-10-18): `check` / `fix` subcommands with `--json` and `--verbose`, exit codes 0 / 1 / 2; `check` runs in parallel and prints at the end, `fix` runs in sequence and prints each step as it ends; tool versions in the header (from `--version`).
   - [x] Tests: `app/tests/verbs.rs` runs the built binary with fake `tomet` / `twrit` scripts on a `PATH` the test controls (10 tests: pass, fail-but-continue, tool missing, verbose, path passed, JSON, fix order, usage error). Whole workspace: tests, clippy `-D warnings`, `twrit check .` clean; the Nix package builds with its tests.
   - Seen when running against the real tools: the `tomet` in the dev shell comes from the flake pin and has no `--version` yet, so its header says "version unknown" until the pin moves past the commit that adds it.
3. [x] Pass-through `sisho <tool> ...` (done 2026-10-18): clap `external_subcommand` resolved through `Tool::from_name`, so a new tool in `Tool::ALL` is picked up with no new variant; `Step::run_inherited` in `sisho-run`; the tool's exit code becomes sisho's (127 not found, 126 could not start, 128+signal, as `env`); an unknown name exits 2 and lists the tools; `--help` shows the tool list built from `Tool::ALL`. 7 integration tests (arguments and flags unchanged, exit code, stdin handed over, not installed, unknown name, help). The cargo-style `sisho-foo` lookup stays deferred.
4. Vault commands moved out of tomet: `new`, `export`, `check-links`, `refactor`, `serve`, `tui`.
5. `sisho convert --to/--from`.
6. Removal from tomet and fixing the callers (twrit `just docs`, tomet writ guards, flakes, `tomet-book`).
