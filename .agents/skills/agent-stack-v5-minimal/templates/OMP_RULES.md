# Agent Stack — sticky OMP rules

- Optimize for iterations-to-verified, not token count.
- For non-trivial changes: planner -> executor -> verifier.
- Prefer Graphify for large-repo grounding before broad repetitive grep/search.
- Use Archify when architecture/runtime/data flow is materially affected or unclear.
- For UI work: UI UX Pro Max before implementation; Impeccable on the rendered result after implementation.
- First unexpected regression: reproduce and prove root cause before another edit.
- Second unexpected regression: stop and replan. Do not stack speculative patches.
- Material deviation from the approved plan requires replanning.
- Do not claim completion without requirement-level evidence.
