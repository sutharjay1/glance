# ADR 0005 — Track work in Linear + repo-native markdown

**Status:** Accepted (2026-07-22)

## Context
A 6-week, 6-phase build needs a tracking system navigable by **both the human and the agent**.
Pure-external (Linear/Notion) drifts from the code and the agent must leave the repo to read state;
pure-repo lacks a good live board/UI for the human.

## Decision
**Two layers:**
- **Durable = repo markdown** (source of truth, git-tracked): `ROADMAP.md`, `docs/adr/`,
  `docs/parity-checklist.md`, `docs/STATUS.md`. The agent reads these with plain file tools.
- **Live = Linear** (team "Jay", project `glance`): one issue per roadmap task, cycles per week.
  The agent files/updates issues via MCP as it works.
- **Query = graphify**: index `docs/` + code into a knowledge graph for natural-language navigation.

## Consequences
- ✅ Agent never leaves the repo to know state; human gets a real board + history.
- ✅ Decisions and parity live in git next to the code.
- ➖ Two places to keep roughly in sync — mitigated by treating `ROADMAP.md` as canonical and Linear as execution.
