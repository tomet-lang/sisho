<!-- Generated from tmtroot/agents.tmt. Edit that, then `tomet export .`. -->

# Sisho

## Core Philosophy & Principles

### Precedence of Instructions

The rules in this document serve as the project baseline and defaults. Explicit instructions given by the user in a conversation always take precedence over these defaults.

### User-First Design over Developer Convenience

Always prioritize the end-user experience, syntax ergonomics, and API design.
If there is tension between developer or implementation convenience and user simplicity, user simplicity must always win. Never compromise user experience just because an implementation is easier.

### Resist Path-of-Least-Resistance & Legacy Bias

The existing implementation is never a justification or reason for new design choices.

- Do not justify decisions with "it is already implemented this way, so let's stick with it."
- Prioritize architectural beauty, elegance, and long-term correctness over the path of least resistance.
- Understand the existing context thoroughly, but never take shortcuts or settle for lazy patches.

### Transparent Execution

Always state what you intend to do before doing it. Never execute non-trivial actions or commands silently.

## Single Source of Truth & Documentation

### Never edit `.md` directly

All Markdown files (`README.md`, `AGENTS.md`) are generated build artifacts, not source files. `CLAUDE.md` is the one exception: it holds only the line `@AGENTS.md`.

- Never edit a generated `.md` file directly.
- Always edit the corresponding `.tmt` file under `tmtroot/`.
- Run `just docs` (or `tomet export .`) to regenerate the Markdown artifacts, and verify with `just docs-check`.
- Direct edits to generated `.md` files will be overwritten and will fail lint checks.

## What this is

`sisho` is the orchestrator CLI for the Tomet ecosystem: one entry point over `tomet`, `twrit` and `tandr`. The design memo is `tmtroot/dev/design.tmt`; read it before changing anything about what `sisho` owns and what it delegates.

The rules that follow from that memo:

- `sisho` has no manifest of its own. Each tool carries its own. If something seems to need a `sisho` setting, treat it as a design question first.
- The core tools are called as subprocesses, never linked as crates. Only what `sisho` implements itself (conversions, and the vault commands moved out of `tomet`) uses `tomet` crates, through git dependencies.
- `sisho` is not a task runner. It does not execute arbitrary commands. The only programs it starts are the tools it knows by name (`tomet`, `twrit`, `tandr`, `tomet-shelf`) and, for `sisho <word>`, a program named `sisho-<word>` on `PATH` (the contract is in `tmtroot/spec/cli/extension.tmt`). Nothing in a vault, a config or a manifest names a program to run.

Read `CONTRIBUTING.md` for the contribution workflow, commit conventions, and verification steps.

## Changes in tomet

The `tomet-*` crates are git dependencies, pinned by `Cargo.lock`. A change in the sibling `../tomet` checkout reaches this repository by being pushed there, then taken with `cargo update -p <crate>`.

Never point the crates at a local checkout with `[patch]` in `.cargo/config.toml`. Cargo then writes the path into `Cargo.lock` and drops the `source` of the patched crate, which the Nix build cannot fetch.

`tomet` is also pinned a second time, by `flake.lock`, for the `tomet` binary in the dev shell (`nix flake update tomet`); the two pins are not checked against each other yet (see the design memo). Move them together.

## Language

Write code comments and documentation in English. Do not use Japanese in code.

## Environment & Tooling

All commands, builds, and tests must run within the Nix development environment (via `direnv` or `nix develop`). Assume this environment is active and rely on the tools provided by the flake, rather than assuming or relying on global host toolchains.

## Task tracking & Workspaces

Before starting implementation on any non-trivial task, create a file under `.agents/tasks/` (one per task, e.g. `.agents/tasks/<short-task-slug>.md`) breaking the work into discrete steps. Update it immediately after each step. Keep it accurate enough that the work can be resumed from that file alone, without the conversation that produced it.

For non-trivial tasks, isolate work in a dedicated Jujutsu workspace under `.agents/workspaces/`:

- Create the workspace before starting edits: `mkdir -p .agents/workspaces && jj workspace add .agents/workspaces/<short-task-slug>`
- Perform edits, builds, and verification within that workspace directory.
- Minor tasks (such as one-off typos or trivial single-file tweaks) may be performed directly in the default workspace.
- When the task is completed and verified, clean up the workspace: `jj workspace forget <short-task-slug>` and remove the directory `rm -rf .agents/workspaces/<short-task-slug>`.

When the task is done, fold anything worth keeping into where it belongs -- a doc comment, the design memo, a commit message -- and then delete the task file.

## Commits & Version Control (jj)

