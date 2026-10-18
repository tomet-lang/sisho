# Sisho development tasks.

default:
    @just --list

build:
    cargo build --workspace

test:
    cargo test --workspace

# Generate and update all derived documentation across the workspace.
docs:
    tomet export .
    tomet format -i .

# Check that all documents parse, format cleanly, and match export targets.
docs-check:
    tomet check .
    tomet format --check .
    tomet export --check .
