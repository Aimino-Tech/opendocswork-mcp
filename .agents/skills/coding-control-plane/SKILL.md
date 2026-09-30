---
name: coding-control-plane
description: Plan, execute, and independently verify non-trivial coding work with strong repository grounding, explicit acceptance criteria, anti-regression gates, and strict limits on fix loops. Use for coding tasks where first-pass correctness matters more than token cost.
---

# Coding Control Plane

Use this workflow for non-trivial coding changes. Optimize for **few iterations to verified**, not for short prompts or low token usage.

## Operating model

Run three phases in order unless the user explicitly requests only one:

1. **PLAN** — understand the repository and produce an implementation/verification contract.
2. **EXECUTE** — implement the approved plan without redesigning the task mid-loop.
3. **VERIFY** — independently prove each acceptance criterion and detect regressions.

For small obvious edits, compress the phases, but keep the same discipline.

## PLAN

Before writing code:

- Restate the desired observable outcome.
- Inspect the relevant repository paths, tests, schemas, configs, APIs, and runtime flow.
- Prefer structural evidence (symbol/call/dependency graph, architecture docs, code navigation) before broad repetitive grep/search when available.
- Identify affected components and adjacent regression surfaces.
- State invariants: behavior that must not change.
- Record assumptions and the evidence that validates them.
- For UI work, invoke the installed UI/UX skill before implementation and produce a UI contract.
- For architecture-sensitive work, make the before/after flow explicit.

Produce:

- Goal
- Requirements `R1`, `R2`, ...
- Evidence consulted
- Affected components/files
- Invariants / must-not-change behavior
- Assumptions and validation
- Architecture impact
- Implementation steps in dependency order
- Acceptance criteria mapped to requirements
- Verification plan with concrete evidence/commands
- Regression surface
- UI contract when relevant
- Open decisions / missing evidence
- Non-goals

Then challenge the plan: what dependency, assumption, behavior, or simpler implementation might have been missed? Revise before execution.

End planning with `PLAN_READY`, `NEEDS_EVIDENCE`, or `HUMAN_DECISION`.

## EXECUTE

Implement the plan, not a new design discovered while coding.

Before edits, confirm the current repo still matches the plan. Change the smallest coherent file set. Reuse existing components, APIs, and abstractions. Avoid opportunistic refactors.

For UI work, follow the UI contract and the existing product language. Implement real states: loading, empty, error, disabled, selected, responsive behavior where relevant.

### Anti-hamster-wheel protocol

An unexpected failure is evidence, not permission to patch randomly.

**First unexpected failure/regression:**
- reproduce it;
- classify it as current-change, pre-existing, environment, or plan-invalidating;
- trace it to a concrete cause;
- make one surgical fix only when the cause is proven local.

If the cause is not proven local, stop and return `REPLAN`.

**Second unexpected regression:** stop implementation. Do not stack another speculative patch. Return the evidence and the plan assumption that appears wrong. Status: `REPLAN`.

Any material deviation — new service, persistence model, API contract, broad refactor, or new UI direction — requires replanning.

End execution with `IMPLEMENTED_FOR_VERIFICATION`, `REPLAN`, or `HUMAN_DECISION`.

## VERIFY

Prefer a fresh context when possible. Do not treat executor claims as proof.

For every requirement / acceptance criterion record `PASS`, `FAIL`, or `UNPROVEN` and attach reproducible evidence: tests, commands, code path, runtime behavior, API result, screenshot/render, or other proof. `UNPROVEN` is not `PASS`.

Verify:

- planned behavior;
- callers/downstream consumers;
- API/schema compatibility;
- loading/error/empty/state transitions;
- auth/permissions when relevant;
- build/type/test integrity;
- unintended files or behaviors changed;
- architecture delta;
- rendered UI for UI-facing changes.

For UI, invoke the UI review skill and inspect the actual rendered result. Name each observed defect before proposing a fix. Do not enter random CSS tweaking.

Return exactly one status:

- `READY` — every material criterion is proven and no blocking regression remains.
- `FIX_LOCAL` — one narrow, proven defect; specify the exact fix and re-verification.
- `REPLAN` — false plan assumption, broader scope required, repeated regression, or missing/contradictory requirements.
- `HUMAN_DECISION` — a genuine product/architecture/risk decision is unresolved.

## Syntaro mode

When the task is for Syntaro planning/verification, make PLAN and VERIFY stricter:

- distinguish implementation questions from CTO/product decisions;
- surface missing evidence rather than filling gaps silently;
- require explicit requirement-to-evidence mapping;
- require architecture compliance for changed flows;
- treat the executor as replaceable; the plan and verification artifacts are the stable contract;
- never mark work ready while a material decision or evidence gap remains.

## Output templates

Use `assets/plan-template.md` for planning handoff and `assets/verification-template.md` for the final verification report when helpful.
