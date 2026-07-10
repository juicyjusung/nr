# Domain Docs

How engineering skills consume this repo's domain documentation.

## Before exploring, read these

- `CONTEXT.md` at the repo root, or
- `CONTEXT-MAP.md` if it exists.
- Relevant ADRs under `docs/adr/`.

If files do not exist, proceed silently. Domain-modeling skills create them lazily when terms or decisions are resolved.

## File structure

Single-context repo:

```
/
├── CONTEXT.md
├── docs/adr/
└── src/
```

## Use the glossary's vocabulary

Use terms defined in `CONTEXT.md`. Avoid synonyms the glossary rejects.

## Flag ADR conflicts

Surface conflicts with existing ADRs explicitly instead of silently overriding them.
