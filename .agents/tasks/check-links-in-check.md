# Run `tomet check-links` in `sisho check`

> Cross-repository coordination: `bearings/initiatives/check-links.tmt` (the table and the order live there; the work stays here).

`check-links` went back to tomet on 2026-10-24 (`refactor: leave check-links to tomet`, sisho `58c84d80`; tomet 0.5.0). sisho has no verb for it:
`sisho tomet check-links` reaches it, and `sisho-counter` runs `tomet check-links --json` for its Broken Links pane (`l`).

It is **not** a step of `check` yet, for the reason it never was: the repositories it would run over are red.
2026-10-24, measured with a throwaway cache: tomet 90 broken links, twrit 2, sisho 0.

## Wait for

- [ ] `tomet/.agents/tasks/clean-broken-links.md` done: `tomet check-links .` exits 0 in tomet.
- [ ] twrit: 2 broken links (`.agents/workspaces/` in `tmtroot/agents.tmt:312`, `crates/twrit` in `tmtroot/readme.tmt:27`). Both look like
      references to something that does not exist at that path; fix or remove.
- [ ] Anyone else who runs `sisho check` over a vault (`immermemo`, `tomet-corpus`, `examples`, ...): measure them too,
      because the step makes their `check` red the day it lands. Decide whether that is wanted (it is what the step is for) or each gets
      a clean-up commit first.

## Do

- [ ] `crates/sisho-tools/src/plan.rs`: add `Tool::Tomet, ["check-links"]` to `check_steps` (read-only, so it runs with the others).
      Do **not** reuse `check_links_step`: that one asks for `--json` for the counter; the step of `check` wants the text, whose failure lines
      are what the report shows (`<file>:<line>:<column>: broken link -> <target>`). Decide whether `check --json` should carry the step's
      stdout as it does for the others (it will: stdout is part of every step).
- [ ] Update the tests that say it is not there: `crates/sisho-tools/src/tests.rs` (`check_does_not_run_the_link_check_yet`,
      `check_runs_these_commands_in_this_order`, `step_names_say_what_the_step_does`) and `app/tests/check.rs` (the counts "1 of 5 steps",
      the JSON `steps[...]` indices).
- [ ] `sisho-counter`: the Checks pane lists the steps of `check`, so the link check appears there too; the Broken Links pane keeps its own
      `tomet check-links --json` run (it needs the positions). Decide whether the two should be one run, and then the step has to be `--json`.
      The counter's `l` could also become "re-run that step".
- [ ] Docs: `tmtroot/readme.tmt` ("`check` runs ..."), `tmtroot/dev/design.tmt` (the `check` verb section and "check-links を tomet に戻す"),
      `tmtroot/spec/cli/*` (there is no `check.tmt` yet: see `spec-cli-pages.md`). Run `just docs`.
- [ ] The minimum tomet is already 0.5.0, the version that has the command.
- [ ] `just docs-check` passes, and so does `sisho check .` in tomet and in twrit.

Acceptance: `sisho check` over a vault with a broken link fails with the file, line and column of the link, and passes over one without.
