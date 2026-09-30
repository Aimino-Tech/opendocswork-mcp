---
name: ui-visual-verifier
description: Independently inspect rendered product UI after implementation, classify concrete visual/UX defects, verify responsive and state behavior, and prevent random CSS fix loops. Use after UI changes or when the result looks weak despite correct code.
---

# UI Visual Verifier

Review the **rendered interface**, not just source code. The objective is to find specific observable defects and decide whether the implementation is ready, needs one local fix, or needs design/replanning.

## Evidence first

When possible, inspect screenshots or the running UI at relevant viewports and states. If only source is available, mark visual claims as `UNPROVEN` rather than pretending they were observed.

## Review order

1. **Task clarity** — can the user identify current state, priority, and next action quickly?
2. **Hierarchy** — are emphasis and grouping proportional to importance?
3. **Layout** — alignment, rhythm, whitespace, density, scrolling, overflow.
4. **Typography** — readable sizes/line lengths, consistent roles, no accidental visual noise.
5. **Components** — consistent controls, radii, elevation, icons, states.
6. **Color/material** — semantic contrast, glass/translucency legibility, no effect overload.
7. **Interaction states** — hover/focus/selected/disabled/loading/error/empty as relevant.
8. **Responsive behavior** — inspect at least desktop and narrow/mobile when applicable.
9. **Accessibility** — obvious contrast, focus, semantics/labels, target size, motion issues.
10. **Product consistency** — does this look like the same product as adjacent surfaces?

Use `references/defect-taxonomy.md` to classify findings.

## Anti-loop rule

Never recommend “tweak spacing/colors until it feels right.” Each fix must name:

- the observed defect;
- evidence/location;
- likely cause;
- smallest correction;
- what must be re-checked afterward.

If two visual fixes reveal new unrelated structural defects, stop and return `REPLAN_UI` instead of continuing to patch.

## Status

Return one:

- `UI_READY`
- `FIX_LOCAL`
- `REPLAN_UI`
- `NEEDS_RENDER_EVIDENCE`

Use `assets/ui-review-report.md` when a structured report helps.

## Optional local Impeccable integration

If the project already has the Impeccable CLI/hook installed and shell access is available, its detector output can be used as additional evidence. Do not assume it ran. A clean detector result is not a substitute for inspecting hierarchy, usefulness, or the rendered experience.
