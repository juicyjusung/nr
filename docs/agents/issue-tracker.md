# Issue tracker: GitHub

GitHub Issues is the single source of truth for this repo's issues, plans, PRDs, scope, acceptance criteria, decisions, blockers, and final verification. Use the `gh` CLI for all operations.

Local Markdown files may contain supporting notes or derived documentation, but they are not the tracker SSOT. When project truth changes, update the owning GitHub issue.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v` — `gh` does this automatically inside the clone.

## Pull requests as a triage surface

**PRs as a request surface: no.**

When changed to `yes`, external PRs use the same labels and states as issues:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>`.
- **List external PRs**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments`, retaining only `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE`.
- **Comment / label / close**: `gh pr comment`, `gh pr edit`, `gh pr close`.

GitHub shares one number space across issues and PRs. Resolve ambiguous `#42` with `gh pr view 42`, then fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

Used by `/wayfinder`. The map is one issue with child issues as tickets.

- **Map**: issue labelled `wayfinder:map`, containing Notes, Decisions-so-far, and Fog.
- **Child ticket**: GitHub sub-issue. If unavailable, use a task list plus `Part of #<map>`.
- **Blocking**: GitHub native issue dependencies. If unavailable, use `Blocked by: #<n>`.
- **Frontier query**: first open, unblocked, unassigned child in map order.
- **Claim**: `gh issue edit <n> --add-assignee @me`.
- **Resolve**: comment, close, then append context pointer to the map's Decisions-so-far.
