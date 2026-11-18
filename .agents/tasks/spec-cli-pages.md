# The CLI spec pages: five are empty files

`tmtroot/spec/cli/` is where each command's behaviour is written (what it guarantees, where it writes, when it fails), beside the option list that
`sisho spec cli` generates (`tmtroot/generated/cli.tmt`). On 2026-10-24 these five files are 0 bytes:

| file | whose command | what to do |
|---|---|---|
| `convert.tmt` | sisho's verb (a thin entry to six `tomet` commands) | **write it** |
| `fix.tmt` | sisho's verb | **write it** |
| `new.tmt` | `tomet new` (not sisho's: `sisho tomet new`) | delete; tomet has its own page |
| `export.tmt` | `tomet export` | delete; tomet has its own page |
| `refactor.tmt` | `tomet refactor` | delete; tomet has its own page |

`check.tmt` does not exist at all, and `check` is sisho's main verb. `serve.tmt`, `counter.tmt` and `extension.tmt` are written.
(`check-links.tmt` moved to tomet with the command: `tomet/tmtroot/spec/cli/check-links.tmt`.)

## Steps

- [ ] Find out why they are empty (`jj log` of each path): were they ever written, or always placeholders? The history may hold text
      worth keeping.
- [ ] Delete `new.tmt`, `export.tmt`, `refactor.tmt`. Check nothing links to them (`grep -rn 'spec/cli/\(new\|export\|refactor\)' tmtroot`), then
      `tomet check .`.
- [ ] Write `convert.tmt` (the model: `tomet/tmtroot/spec/cli/check-links.tmt`, `serve.tmt` here). What `sisho` owns, and only that:
      the format names (`--to md|commonmark|html|typst|pandoc`, `--from md|commonmark|pandoc`, any case), the command lines it turns
      away before starting `tomet` (`--advanced`/`--lang`/`--body` outside `--to html`; `--remove-original` with `--to`, or without
      `--out`/`--in-place`; both `--out` and `--in-place`), exit code 2 for those; then that it runs `tomet to-md|html|to-typst|to-pandoc|
      from-md|from-pandoc` with the terminal handed over (pipes, output and exit code are `tomet`'s), 127 if `tomet` is not on `PATH`,
      and a pointer to `tomet/tmtroot/spec/cli/overview.tmt` ("変換コマンドの入力と出力先") for the modes. Minimum tomet 0.5.0.
- [ ] Write `fix.tmt`: the steps and their fixed order (`twrit check --fix`, `tomet export`, `tomet format -i`), serial because they write,
      the same failure rules as `check`, exit codes 0/1/2, the version check that stops a `fix` that would have an older tool rewrite files,
      `--json`, `-v`. Source of truth: `app/src/commands/fix.rs`, `crates/sisho-tools/src/plan.rs`, the design memo ("check / fix の実行モデル").
- [ ] Write `check.tmt` the same way (it is the sibling of `fix`); `tmtroot/readme.tmt` already has the prose to start from.
- [ ] Each page ends with "Where it differs from what it should be" and "Not decided", as `serve.tmt` does; do not leave the sections out when
      they are empty (write "none").
- [ ] `just docs-check`.

Acceptance: no 0-byte file under `tmtroot/spec/`; `sisho check .` passes.
