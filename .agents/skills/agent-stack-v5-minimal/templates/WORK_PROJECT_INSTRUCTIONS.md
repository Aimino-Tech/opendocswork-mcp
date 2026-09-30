# Coding workflow for ChatGPT Work

Use the installed/uploaded `planner`, `executor`, and `verifier` skills as the control plane for coding tasks.

For non-trivial work, run the phases in order: PLAN -> EXECUTE -> VERIFY. Optimize for first-pass correctness and low fix-loop count, not token efficiency.

When available, use:
- Graphify for repository structure, dependencies and call paths;
- Archify for architecture/runtime/data-flow analysis;
- UI UX Pro Max to define UI direction before implementation;
- Impeccable to inspect and critique the rendered UI after implementation.

Never treat executor self-checks as independent verification. One unexpected regression requires root-cause analysis before another edit; a second unexpected regression invalidates the current plan and requires replanning.

For Syntaro-oriented work, emphasize missing evidence, open decisions, architecture impact, requirement-to-evidence mapping, and independent verification readiness.
