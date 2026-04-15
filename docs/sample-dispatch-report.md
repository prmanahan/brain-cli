# Sample dispatch report

A real snapshot of dispatch-metrics data from the development of
brain-cli and its sibling projects, anonymized for public release.

## What you're looking at

Every row is a real dispatch: an agent was handed a task, ran it to
completion, and the CLI recorded cost, duration, model, and tool-use
counts as a single row in `dispatch_metrics`. This is what that
table looks like once the data has been through a scrubber.

## Anonymization rules

- **Project names** are replaced with stable slugs (`project-a`,
  `project-b`, ...). The mapping is consistent across the whole
  report, so `project-a` always means the same real project.
- **Task titles** are reduced to a category. No real title survives.
- **Agent names** are kept. They match the companion `puck-playbook`
  repo where each role is documented, so readers can see which kind
  of work each agent handles.
- **Cost, model, duration, tool-use, and token counts** are
  preserved verbatim. Rounding or bucketing would defeat the point
  of the report, which is to let you judge whether this kind of
  accounting is useful for your own work.

Nothing in this file is synthetic. Every row was produced by a real
agent run.

## Snapshot

- Rows: 28
- Distinct projects: 5
- Distinct agents: 7
- Distinct models: 4
- Total cost across the sample: $36.73
- Total wall-clock across the sample: 99.1 min

## Dispatches

| Date | Project | Agent | Category | Model | Tier | Cost (USD) | Duration | Tools | Tokens |
|---|---|---|---|---|---|---|---|---|---|
| 2026-04-11 | project-d | Forge | feature implementation | sonnet | t2 | $0.5100 | 2.1m | 16 | 33,779 |
| 2026-04-11 | project-d | Sage | scoped research brief | opus | t1 | $1.6000 | 4.9m | 23 | 53,256 |
| 2026-04-13 | project-a | Rune | feature implementation | sonnet | t2 | $0.3950 | 3.3m | 30 | 65,891 |
| 2026-04-13 | project-a | Warden | code review | sonnet | t2 | $0.2820 | 1.5m | 9 | 47,025 |
| 2026-04-13 | project-a | Glitch | test authoring | sonnet | t2 | $0.8240 | 12.1m | 147 | 137,270 |
| 2026-04-13 | project-d | Sage | scoped research brief | opus | t1 | $1.9720 | 5.8m | 24 | 65,741 |
| 2026-04-13 | project-a | Glitch | test authoring | sonnet | t2 | $0.2770 | 2.0m | 21 | 46,185 |
| 2026-04-13 | project-a | Bolt | feature implementation | sonnet | t2 | $0.2480 | 2.8m | 7 | 41,331 |
| 2026-04-13 | project-a | Bolt | feature implementation | sonnet | t2 | $0.2780 | 1.6m | 20 | 46,350 |
| 2026-04-14 | project-a | Rune | feature implementation | claude-sonnet-4-6 | t2 | $0.6613 | 1.5m | 15 | 44,089 |
| 2026-04-14 | project-a | Glitch | test authoring | claude-sonnet-4-6 | t2 | $0.6589 | 1.2m | 6 | 43,925 |
| 2026-04-14 | project-c | Ink | resume tailoring | claude-opus-4-6 | t1 | $5.2400 | 3.2m | 26 | 69,917 |
| 2026-04-14 | project-c | Ink | revision pass | claude-opus-4-6 | t1 | $4.1033 | 1.7m | 10 | 54,710 |
| 2026-04-14 | project-a | Rune | feature implementation | claude-sonnet-4-6 | t2 | $0.5643 | 1.3m | 11 | 37,618 |
| 2026-04-14 | project-a | Rune | feature implementation | claude-sonnet-4-6 | t2 | $0.7372 | 2.0m | 26 | 49,146 |
| 2026-04-14 | project-a | Rune | feature implementation | claude-sonnet-4-6 | t2 | $0.5477 | 46.7s | 9 | 36,512 |
| 2026-04-14 | project-c | Ink | revision pass | claude-opus-4-6 | t1 | $0.5796 | 1.6m | 11 | 38,641 |
| 2026-04-14 | project-b | Bolt | spec review | claude-sonnet-4-6 | t2 | $0.4280 | 4.1m | 31 | 47,558 |
| 2026-04-14 | project-b | Glitch | spec review | claude-sonnet-4-6 | t2 | $0.3440 | 3.8m | 11 | 38,193 |
| 2026-04-14 | project-b | Warden | spec review | claude-opus-4-6 | t1 | $2.3690 | 3.1m | 10 | 52,637 |
| 2026-04-14 | project-b | Sage | revision pass | opus | t1 | $4.1800 | 5.9m | 11 | 92,887 |
| 2026-04-14 | project-b | Sage | scoped research brief | claude-opus-4-6 | t1 | $3.6300 | 6.2m | 30 | 80,645 |
| 2026-04-14 | project-b | Sage | scoped research brief | claude-sonnet-4-6 | t2 | $0.3400 | 13.0m | 25 | 38,150 |
| 2026-04-14 | project-b | Sage | scoped research brief | claude-sonnet-4-6 | t2 | $0.3300 | 1.1m | 22 | 36,360 |
| 2026-04-14 | project-b | Warden | spec review | claude-sonnet-4-6 | t2 | $0.5300 | 3.3m | 16 | 59,160 |
| 2026-04-14 | project-b | Bolt | spec review | claude-sonnet-4-6 | t2 | $0.5800 | 3.0m | 22 | 64,448 |
| 2026-04-14 | project-b | Glitch | spec review | claude-sonnet-4-6 | t2 | $0.5400 | 2.8m | 17 | 59,662 |
| 2026-04-15 | project-h | Sage | scoped research brief | claude-opus-4-6 | t1 | $3.9800 | 3.5m | 18 | 132,773 |

## How to read the columns

`Tier` records the routing decision the orchestrator made for the
dispatch. Tier one is the expensive default for deep work, tier two
is the standard working tier, and further tiers cover cheaper cases.
Cost and duration are what they cost to actually run, not estimates.
Token counts are the total in plus out for the dispatch as reported
by the model provider. A dash in the tier column means the dispatch
predates the tier field.

## A few things a reader might notice

Agents are not interchangeable. A scoped research brief from a
senior researcher on an Opus-class model lands in a different cost
and duration bracket from a mechanical refactor on a Sonnet-class
model, and the table makes that legible at a glance. Review-class
work (code review, spec review, design review) tends to be cheap
and fast. Feature implementation is the bulk of the cost.

Dispatches that spend a lot of tokens are not always the expensive
ones. A long-running agent with many tool uses can rack up duration
without matching token count, and vice versa. The four columns are
independent signals and worth tracking independently.
