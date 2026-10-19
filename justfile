# Sisho development tasks.

default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace

# Generate and update all derived documentation across the workspace.
# Runs this checkout's own `sisho`: the dev shell does not carry one.
docs:
    cargo run -q -p sisho-cli -- fix .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    cargo run -q -p sisho-cli -- check .
