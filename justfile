# Sisho development tasks.

# Tools in the Tomet ecosystem pinned as flake inputs.
tool_inputs := "catasto sisho-desk tomet tomet-shelf twrit"

default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace

# Update ecosystem tool inputs in flake.lock (defaults to all tools: catasto, sisho-desk, tomet, tomet-shelf, twrit).
update-tools *inputs:
    nix flake update {{ replace(if inputs == "" { tool_inputs } else { inputs }, "fs-catasto", "catasto") }}

alias flake-update := update-tools

# Generate and update all derived documentation across the workspace.
# Runs this checkout's own `sisho`: the dev shell does not carry one.
docs:
    cargo run -q -p sisho-cli -- spec --out tmtroot/generated
    cargo run -q -p sisho-cli -- fix .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    cargo run -q -p sisho-cli -- check .
    cargo run -q -p sisho-cli -- spec --check --out tmtroot/generated
