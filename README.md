<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Sisho

Orchestrator CLI for the Tomet ecosystem: one entry point over `tomet`, `twrit` and `tandr`.

## Overview

`sisho` (司書, "librarian") keeps the whole project in order. It does not replace the tools it calls: `tomet` stays the language core, `twrit` enforces a repository's written rules, and `tandr` searches and rewrites structure. `sisho` runs them together.

It has no manifest of its own. Each tool already carries its own, and a need for more is a sign of a design mistake.

The design memo is [tmtroot/dev/design.tmt](tmtroot/dev/design.tmt).

## Usage

```bash
sisho check [path]   # verify without writing; steps run at the same time
sisho fix [path]     # repair what the tools can; steps run one after another
sisho <tool> ...     # run a tool with the rest of the command line, unchanged
```

Both take `--json`, and `check` and `fix` take `--verbose` to show the output of steps that passed too. One failing step does not stop the others. A tool that is not installed counts as a failure.

Exit codes of `check` and `fix`: 0 when every step passed, 1 when any did not, 2 when `sisho` itself could not do its job (for example a usage error).

`sisho <tool> ...` is a pass-through: the tool gets the terminal and everything after its name, flags included, and its exit code becomes `sisho`'s. A tool that is not installed exits 127, one that cannot be started 126, one ended by a signal 128 plus the signal, as `env` does.

## Workspace Layout

- `app` (`sisho-cli`) -- Main CLI binary (`sisho`). Layer 2.
- `crates/sisho-tools` (`sisho-tools`) -- What `sisho` knows about `tomet`, `twrit` and `tandr`. Layer 1.
- `crates/sisho-vault` (`sisho-vault`) -- Vault and workspace commands moved out of `tomet`, and conversions. Layer 1.
- `crates/sisho-serve` (`sisho-serve`) -- `serve`, kept apart so the HTTP stack stays out of other builds. Layer 1.
- `crates/sisho-tui` (`sisho-tui`) -- `tui`, kept apart for the same reason. Layer 1.
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

