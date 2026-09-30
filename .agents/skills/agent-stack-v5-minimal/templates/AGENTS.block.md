<!-- agent-stack-v5:start -->
## Coding control plane

Optimize for **iterations-to-verified**, not token count. For non-trivial coding work, use the three role skills in order:

1. `planner` — ground in the repo, define impact/acceptance/verification, and challenge the plan before edits.
2. `executor` — implement the approved plan without scope drift; root-cause unexpected failures before patching.
3. `verifier` — independently prove requirements, regressions, architecture, and rendered UI before declaring done.

Use upstream capabilities instead of reimplementing them:
- **Graphify** for repository graph, call paths, dependencies, and impact discovery.
- **Archify** for architecture/runtime/data-flow modeling when boundaries or flows matter.
- **UI UX Pro Max** before UI implementation to establish design direction and constraints.
- **Impeccable** after UI implementation to inspect the rendered result and detect design problems.

Hard rules:
- Do not skip verification for non-trivial changes.
- Do not optimize for fewer tokens or fewer reads; optimize for fewer wrong edits and fix loops.
- One unexpected regression: diagnose root cause before another edit.
- Two unexpected regressions: stop patching and return to planning.
- A material plan deviation requires replanning rather than silent improvisation.
- UI changes require a design contract before coding and rendered verification afterward.
- `READY` means evidence-backed acceptance criteria, not “the code looks fine.”

Project profile is stored in `.agent-stack/profile` (`coding` or `syntaro`). The same three skills are used in both modes; `syntaro` makes planning and verification stricter.
<!-- agent-stack-v5:end -->
