# Attribution

## Lineage

brain-cli is the task and dispatch tracker that produces the numbers a
forthcoming article on agent-first software workflows will draw from. The
CLI started as an internal tool, and this repository is the sanitized
snapshot that ships ahead of that article as a standalone reference. It is
released alongside
[puck-playbook](https://github.com/prmanahan/puck-playbook), which
documents the orchestration practices the CLI was built to support.

## Upstream code and writing

No upstream code or writing is incorporated into this repository. No
`NOTICE`, `LICENSE`, or `Adapted from:` markers were found in any shipped
source file during the pre-publication scan.

## Dependencies

All runtime and dev dependencies are declared in `Cargo.toml` files and
resolve through `Cargo.lock`. Licenses are allowlisted in `deny.toml` and
checked in CI via `cargo-deny`.
