# brain-cli

A small Rust command-line tool for tracking tasks, dispatches, and agent
activity against a local SQLite database. It is the accounting layer behind
a forthcoming article on agent orchestration, released here as a
point-in-time snapshot so readers can clone it, build it, and see what the
real thing looks like.

## Platform note

brain-cli was developed on macOS. Rust is portable, so it will very likely
build on Linux with no changes, but **the tool is not cross-platform
tested**. There is no Windows support, no CI matrix covering other
operating systems, and no packaging for anything beyond a local `cargo
build`. Treat it as a macOS-first reference implementation that happens to
be written in portable Rust.

## Lineage

brain-cli is the tool that produces the dispatch-cost numbers and the
task-state transitions that a forthcoming article on agent orchestration
will draw from. This repository goes out ahead of that article as a
standalone artifact, so the numbers in it exist on disk before they show
up in prose. It is the scratchpad an orchestrator (the "Puck" framework)
writes to when it dispatches work, and the audit trail it reads from when
it needs to know what the team has been doing.

It is a **frozen artifact**. There is no ongoing merge relationship with
the internal tree it was extracted from. Bug fixes land on the internal
copy first and may or may not be backported; the public release is a
snapshot, not a branch. If you want the companion release that documents
the orchestration practices around it, see
[puck-playbook](https://github.com/prmanahan/puck-playbook).

## Quick start

```sh
git clone https://github.com/prmanahan/brain-cli.git
cd brain-cli
cargo build
./target/debug/brain --db /tmp/demo.db task list
```

The first run creates the database file, initializes the schema, and
returns an empty task list. You can then try:

```sh
./target/debug/brain --db /tmp/demo.db task create "first task"
./target/debug/brain --db /tmp/demo.db task list
./target/debug/brain --db /tmp/demo.db dispatch list
```

Every subcommand takes a `--db <path>` flag. There is no global config
file, no environment-variable fallback, and no implicit default outside
the current directory. If you forget the flag, the CLI will tell you.

## Schema overview

The database has seven tables. They are created in a single FK-safe
`ensure_schema()` call the first time a `Database` is opened.

| Table | What it holds |
|---|---|
| `projects` | A short registry of project names and optional paths |
| `tasks` | Task rows: id, title, status, assignee, parent, project, timestamps |
| `dispatch_metrics` | One row per agent dispatch: cost, model, duration, tokens, tool calls |
| `dispatch_events` | Event-level timeline inside a single dispatch |
| `activity_log` | Free-form activity timeline across the workspace |
| `session_events` | Session start/stop and context markers for the orchestrator |
| `scope_violations` | Agent-scope boundary violations captured by hook scripts |

The shapes are small. A task is not a ticket. A dispatch is not a thread.
The goal is just enough structure to make the accounting honest, not to
build a general-purpose project tracker.

## Shape notes

brain-cli started as exploratory work and is still an early prototype. What
follows is observation, not deliberate design.

**Schema in code.** There is no migrations directory and no external
schema file. All seven `CREATE TABLE` statements live in a single method
on the `Database` struct, run idempotently on `open()` and `in_memory()`.
That choice costs flexibility once the tool is in wide deployment. It buys
simplicity while it is not. A fresh clone can create a fresh database and
work, and a test can spin up an in-memory database with one call.

**Commands are hardcoded.** The subcommand surface is a static `clap` enum
and a match block in `main.rs`. There is no plugin system, no dynamic
dispatch, no configuration DSL for new verbs. If you want a new command,
you edit Rust and rebuild. That is the same trade as the schema choice:
simple until it isn't, and this is the until-it-isn't phase.

**Frozen artifact, not a product.** brain-cli is published as a companion
to an article, not as a tool under active development. Issues and pull
requests are welcome as documentation, but the intent is not to grow the
public copy into a maintained release train. The place where ideas from
this tool go next is a successor tool, not this repository.

## Sample dispatch report

A real, anonymized snapshot of `dispatch_metrics` data lives at
[`docs/sample-dispatch-report.md`](docs/sample-dispatch-report.md). Project
names are replaced with stable slugs, task titles are reduced to
categories, and agent names, models, costs, and durations are preserved
verbatim. The sample is there so a reader can decide whether this kind of
per-dispatch accounting is useful in their own workflow before cloning
anything.

## Roadmap

The roadmap is short and points at a successor, not at this repository.

A successor tool is in progress that will replace the hardcoded-command
layer with a pluggable one, add a real migration system so the schema can
evolve without a rewrite, and introduce multi-user authentication so the
same data can back more than a single operator. Those are the reasons the
current surface is locked: the next iteration is large enough that
shipping incremental changes on the old shape would make both versions
worse. When the successor ships, this repository will stay where it is as
the reference for what came before.

Inside the scope of brain-cli itself, the only open follow-up is whatever
bugs the public release surfaces before it is left to rest.

## Building and testing

Standard Rust workflow. The minimum commands are:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

Continuous integration runs the same set plus `cargo-audit`, `cargo-deny`,
`cargo-llvm-cov`, and CodeQL. Coverage is reported but not gated on day
one; the bar will be raised once the post-release baseline settles.

## License and attribution

MIT. See [LICENSE](LICENSE). Copyright (c) 2026 Peter Manahan.

Attribution and lineage live in [ATTRIBUTION.md](ATTRIBUTION.md). The
security reporting policy lives in [SECURITY.md](SECURITY.md). A companion
article is forthcoming; the companion repository is
[puck-playbook](https://github.com/prmanahan/puck-playbook).
