<!-- Generated from tmtroot/readme.tmt. Edit that, then `tomet export .`. -->

# Sisho

Orchestrator CLI for the Tomet ecosystem: one entry point over `tomet`, `twrit` and `tandr`.

## Overview

`sisho` (司書, "librarian") keeps the whole project in order. It does not replace the tools it calls: `tomet` stays the language core, `twrit` enforces a repository's written rules, and `tandr` searches and rewrites structure. `sisho` runs them together.

It has no manifest of its own. Each tool already carries its own, and a need for more is a sign of a design mistake.

The design memo is [tmtroot/dev/design.tmt](tmtroot/dev/design.tmt).

## Workspace Layout

- `app` (`sisho-cli`) -- Main CLI binary (`sisho`).

## Building and Testing

### Build workspace

```bash
cargo build
```

### Run tests

```bash
cargo test --workspace
```

