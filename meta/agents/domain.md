# Domain Docs

How the engineering skills should consume this repo's domain documentation when exploring the codebase.

## Layout

Single-context. All domain docs live under `meta/`:

```
/
├── CLAUDE.md                ← agent rules (auto-discovered)
└── meta/
    ├── CONTEXT.md           ← domain glossary (lazily created)
    ├── adr/                 ← architectural decisions (lazily created)
    └── agents/              ← per-skill config
```

The `docs/` folder at the repo root is reserved for the docsify documentation site — do not place agent or domain files there.

## Before exploring, read these

- **`meta/CONTEXT.md`** if it exists — domain glossary.
- **`meta/adr/`** if it exists — read ADRs that touch the area you're about to work in.

If any of these files don't exist, **proceed silently**. Don't flag their absence; don't suggest creating them upfront. The producer skill (`/grill-with-docs`) creates them lazily when terms or decisions actually get resolved.

## Use the glossary's vocabulary

When your output names a domain concept (in an issue title, a refactor proposal, a hypothesis, a test name), use the term as defined in `meta/CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

If the concept you need isn't in the glossary yet, that's a signal — either you're inventing language the project doesn't use (reconsider) or there's a real gap (note it for `/grill-with-docs`).

## Flag ADR conflicts

If your output contradicts an existing ADR, surface it explicitly rather than silently overriding:

> _Contradicts ADR-0007 (event-sourced orders) — but worth reopening because…_
