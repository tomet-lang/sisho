# catasto through sisho: what could still be added

> Cross-repository coordination: `bearings/initiatives/catasto-and-sisho.tmt` (the table and the order live there; the work stays here).

Found 2026-10-24 while walking through "adopt catasto in an empty directory using only `sisho catasto ...`". The walk-through works
(init, upstream add, add, list, check, sync --only, remove); what is left is not needed for it.

- [ ] `sisho fix` and `catasto sync`: still out (decided). `catasto sync --only FILE` and `catasto list --json` now exist, so a smaller
  step is possible: sync only the files `list` reports `missing` or `unlocked` (never `modified`). That would make a fresh clone's `fix`
  complete without restoring an edited file. Needs a catasto option (`sync --missing`) first.
- [ ] The counter dashboard (`sisho-counter`) shows tool versions and broken links; it could show `catasto list --json` (how many
  files, how many drifted). Reads the tool's JSON only, no manifest of its own.
- [ ] `sisho check` prints catasto's hint as `catasto sync`; through sisho the command is `sisho catasto sync`. Leave it (sisho does not
  interpret a tool's output), or let `catasto` print the program name it was started as.
- [ ] `sisho check --json` has the step's stderr; nothing parses it. A `kind` or `hint` field in catasto's own JSON output (`catasto check
  --json`) would let a tool say more than "exit 1".
