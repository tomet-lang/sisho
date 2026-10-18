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

1. Decide the layer structure of the crates (twrit enforces layering through a writ; sisho should settle its own before adding crates), then add `.writ.tmt`.
2. Subprocess runner and the `check` / `fix` verbs for tomet and twrit.
3. Pass-through `sisho <tool> ...`.
4. Vault commands moved out of tomet: `new`, `export`, `check-links`, `refactor`, `serve`, `tui`.
5. `sisho convert --to/--from`.
6. Removal from tomet and fixing the callers (twrit `just docs`, tomet writ guards, flakes, `tomet-book`).
