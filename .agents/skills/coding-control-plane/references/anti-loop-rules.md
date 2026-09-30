# Anti-loop rules

Use these as hard gates when an implementation begins to thrash.

- Reading/analysis can be extensive. Writing should be deliberate.
- A failing test does not automatically justify an edit.
- Diagnose before changing code.
- One proven local cause permits one surgical fix.
- A second unexpected regression invalidates the current execution path: stop and replan.
- Repeated CSS tweaks without a named rendered defect are prohibited.
- Repeated edits outside the planned file set require a fresh impact analysis.
- “Tests pass” is not equivalent to “requirement proven.”
- Verification should be independent from implementation when the harness permits it.
