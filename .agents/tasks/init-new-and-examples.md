# sisho init, sisho new and template scaffolding (examples)

Discussion on 2026-11-04 regarding vault initialization and project scaffolding.

Currently, `examples/` in the project-tomet root hosts reference vault layouts (`pkm/`, `app-dev/`, `wiki/`), but users must copy-paste them manually. While `tomet new` creates single documents from blueprints, whole-vault initialization (`default.config.tmt`, `.writ.tmt`, `catasto.ctmt`, folder structures) belongs to the orchestrator `sisho` (analogous to `cargo new` / `cargo init`).

`tomet-corpus` / `tomet-synth` remain separate developer/testing tools (exposed as the `sisho-corpus` extension), keeping user-facing template scaffolding clean.

## Items & Open Decisions

- [ ] CLI command structure:
  - `sisho new <path> [--example <name>]`: create directory and initialize with template.
  - `sisho init [path] [--example <name>]`: initialize existing directory in-place.
- [ ] Template bundling strategy:
  - Option A (recommended): Embed standard templates (`pkm`, `app-dev`, `wiki`) at compile time (e.g. `include_dir!`) for zero-network, offline-first reliability.
  - Option B: Fetch remote git templates.
  - Option C: Hybrid (embedded default templates + optional git URL flag).
- [ ] Scaffolding execution:
  - Copy/expand files.
  - Replace vault/project name placeholders if needed.
  - Safe overwrite checks (require `--force` if directory is non-empty).
  - Ensure generated templates pass `sisho check` cleanly out of the box.
- [ ] Counter dashboard integration:
  - When `sisho` is run without arguments in an uninitialized directory, offer an interactive onboarding screen to pick a template and initialize.
