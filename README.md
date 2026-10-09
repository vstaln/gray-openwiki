# gray-openwiki

Codebase wiki generator plugin for [gray](https://github.com/...) — a Rust
sidecar that ports the pi `openwiki` package's core loop into gray's
minimal non-channel plugin shape.

`/wiki` generates a structured Markdown wiki under `.gray/wiki/` in the
current repository; `/wiki update` refreshes only the sections affected
by source changes, using a mtime+size ledger so unchanged pages are left
alone.

## What it does

The wiki is agent-authored but plugin-managed:

- **`/wiki`** (or `/wiki init`) — the plugin snapshots every tracked
  source file (`git ls-files -co --exclude-standard`, or a recursive walk
  outside git work trees), stages `.gray/wiki/.ledger.pending.json`, and
  returns a prompt (from `prompts/init.md`) that drives the agent to
  write `index.md`, `architecture.md`, `modules/*.md`, `flows/*.md`, and
  `glossary.md`. The prompt ends by running `gray-openwiki commit`, which
  promotes the pending snapshot to `.gray/wiki/.ledger.json`.
- **`/wiki update`** — diffs a fresh scan against the committed ledger.
  Up to date → a one-line say. Stale → stages a new pending snapshot and
  returns a prompt (from `prompts/update.md`) listing only the
  added/modified/removed files plus any missing pages, so the agent
  touches just the affected sections.
- **`/wiki status`** — one-line status. **`/wiki path`** — the wiki dir.
- **Tool `wiki_status({path?})`** — JSON status for the model: presence,
  page list, commit timestamp, HEAD, tracked-file count, drift counts,
  and a hint (`run /wiki update …`).
- **`prompt/context` hook** — when a wiki exists for the session's repo,
  a short note tells the model to consult `.gray/wiki/index.md` before
  deep exploration, and flags a staged-but-uncommitted run.

## Install / register

```sh
cargo build --release
gray plugin install target/release/gray-openwiki   # or register <path>
gray plugin check target/release/gray-openwiki     # conformance check
```

The installed manifest (`gray-openwiki manifest`) declares
`sidecar_args: ["sidecar"]`; the bare binary also runs the wire loop, so
`gray plugin check <binary>` works directly on the build output.

## CLI (debug / standalone)

```
gray-openwiki [sidecar]      NDJSON wire loop (default)
gray-openwiki manifest       install manifest (JSON)
gray-openwiki init|update [path]   stage a run, print the prompt
gray-openwiki status [path]        JSON status
gray-openwiki commit [--root p]    promote .ledger.pending.json
gray-openwiki scan <path>          tracked-file snapshot summary
```

## Files & limits

- `.gray/wiki/` — pages plus `.ledger.json` / `.ledger.pending.json`.
- `.openwikiignore` at the repo root adds repo-relative prefixes to skip.
- File signatures are mtime+size (not content hashes): a rewrite that
  preserves both is treated as unchanged; a `touch` counts as modified.
- Scans cap at 200k files; beyond that drift is reported as unknown
  rather than wrong.
- No network, no secrets, no model calls inside the plugin — the agent
  does the reading and writing; the plugin owns the ledger.
