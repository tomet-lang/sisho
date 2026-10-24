<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Sisho

Orchestrator CLI for the Tomet ecosystem: one entry point over `tomet`, `twrit` and `tandr`.

## Overview

`sisho` (司書, "librarian") keeps the whole project in order. It does not replace the tools it calls: `tomet` stays the language core, `twrit` enforces a repository's written rules, and `tandr` searches and rewrites structure. `sisho` runs them together.

It has no manifest of its own. Each tool already carries its own, and a need for more is a sign of a design mistake.

The design memo is [tmtroot/dev/design.tmt](tmtroot/dev/design.tmt).

## Usage

```bash
sisho                # interactive counter dashboard (when run on a terminal with no arguments)
sisho check [path]   # verify without writing; steps run at the same time
sisho fix [path]     # regenerate, export, format; steps run one after another
sisho check-links [path]   # find links that point at nothing (cached, outside the vault)
sisho convert <path|-> --to F | --from F   # one document, standard input or a directory, to or from Tomet (tomet converts)
sisho serve <file>         # a document as HTML over HTTP on 127.0.0.1, rendered again per request
sisho desk [path]          # interactive desk (extension): migrate Markdown, edit metadata, search structure
sisho --list [--all]       # list installed extensions found on PATH (-l)
sisho <tool> ...     # run a tool with the rest of the command line, unchanged
```

Both take `--json`, and `check` and `fix` take `--verbose` to show the output of steps that passed too. One failing step does not stop the others. A tool that is not installed counts as a failure.

`check` runs `tomet check`, `tomet format --check`, `tomet export --check` and `twrit check`: the gate each repository's `docs-check` spelled out by hand. `fix` runs `twrit check --fix`, `tomet export` and `tomet format -i`, in that order. `check-links` is not part of either: it is a command of its own.

`sisho` needs `tomet` and `twrit` to be at least 0.2.0, the version that gave `--version` its meaning, and reads `--version` of each to know. A tool that is older, or that cannot say its version, is a failed step called `<tool> version` first in the report, which says to update it. `check` goes on with the rest, since it only reads. `fix` writes nothing and exits 1, since it would have an older tool rewrite files. A tool that is not installed is reported by its own steps. Nothing here depends on how the tools were installed.

What belongs to the language stays in `tomet`, which `sisho` reaches with `sisho tomet ...`: `sisho tomet new` makes a document from a blueprint, `sisho tomet refactor` migrates old spellings, `sisho tomet export` writes the outputs a document declares for itself.

Exit codes of `check` and `fix`: 0 when every step passed, 1 when any did not, 2 when `sisho` itself could not do its job (for example a usage error).

Commands that work on the vault itself, like `check-links`, `convert` and `serve`, exit 0 on success and 1 when they could not do what was asked, as `tomet` does; a usage error is 2.

The behaviour of `check-links` and `serve`, which moved here from `tomet`, is written down under `tmtroot/spec/cli/`.

When the reader of the output goes away early (`sisho check-links | head`), `sisho` ends quietly with 141, which is 128 plus `SIGPIPE`, as a shell reports it.
When run with no arguments on an interactive terminal, `sisho` opens its own counter dashboard (`sisho-counter`), showing project health, tool and extension status, and offering on-demand check and fix actions. Running `sisho` without a terminal prints command-line help. The contract is written down under [tmtroot/spec/cli/counter.tmt](tmtroot/spec/cli/counter.tmt).

`sisho <tool> ...` is a pass-through: the tool gets the terminal and everything after its name, flags included, and its exit code becomes `sisho`'s. A tool that is not installed exits 127, one that cannot be started 126, one ended by a signal 128 plus the signal, as `env` does.

`sisho shelf ...` is the same pass-through to `tomet-shelf`, which builds and serves a site from a vault: `sisho shelf build`, `sisho shelf serve`. The word is the role and the program keeps its own name. `shelf` takes no part in `check` or `fix`.

An extension is a program named `sisho-<word>` on `PATH`, run as `sisho <word> ...`. It is the same pass-through: `sisho` hands over the terminal, forwards all arguments and flags, and takes its exit code. Built-in commands and known tools always take precedence over an extension of the same name. An unknown word with no matching extension on `PATH` exits 2. `sisho --list` (or `-l`) lists all installed extensions found on `PATH`; with `--all`, duplicate programs that are shadowed by earlier entries in `PATH` are also shown. The contract is written down under [tmtroot/spec/cli/extension.tmt](tmtroot/spec/cli/extension.tmt).

## With Nix

Nothing in `sisho` needs Nix: it finds `tomet` and `twrit` on `PATH`, however they got there. For those who do use Nix, the flake brings the tools, so a repository imports `sisho` and nothing else for them:

```nix
inputs.sisho.url = "github:tomet-lang/sisho";

# in a dev shell:
packages = [ sisho.packages.${system}.toolchain ];   # sisho, tomet, twrit and sisho-desk
```

Packages are `sisho`, `tomet`, `tomet-lsp`, `twrit`, `tomet-shelf` and `sisho-desk`. `toolchain` bundles the core: `sisho`, `tomet`, `twrit` and `sisho-desk`. To choose your own set, as with fenix:

```nix
sisho.legacyPackages.${system}.combine [
  sisho.packages.${system}.toolchain
  sisho.packages.${system}.tomet-shelf
]
```

Each tool is built as its own flake builds it, so the binary cache of `tomet` serves them. The versions that go together are the ones `sisho`'s `flake.lock` pins; `nix flake update sisho` moves them all at once.

## Workspace Layout

- `app` (`sisho-cli`) -- Main CLI binary (`sisho`). Layer 2.
- `crates/sisho-tools` (`sisho-tools`) -- What `sisho` knows about `tomet`, `twrit` and `tandr`. Layer 1.
- `crates/sisho-vault` (`sisho-vault`) -- The vault commands `sisho` does itself: `check-links`. Layer 1.
- `crates/sisho-serve` (`sisho-serve`) -- `serve` (it runs `tomet html` for each request), kept apart so the HTTP stack stays out of other builds. Layer 1.
- `crates/sisho-counter` (`sisho-counter`) -- Interactive counter dashboard, kept apart so the TUI stack stays out of the core CLI. Layer 1.
- `crates/sisho-run` (`sisho-run`) -- Runs external commands and collects outcomes. The only crate that starts a process. Layer 0.

## Building and Testing

### Build workspace

```bash
cargo build
```

### Run tests

```bash
cargo test --workspace
```

