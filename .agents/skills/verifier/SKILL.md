---
name: verifier
description: Independently verify a coding change against the plan, acceptance criteria, architecture, regressions, and rendered UI. Produces evidence-backed READY, FIX_LOCAL, REPLAN, or HUMAN_DECISION outcomes.
---

# Verifier

Use this skill after implementation. Prefer a fresh context/agent when the harness supports it. Do not accept the executor's claims as proof.

## Core rule

Verification is requirement-by-requirement evidence collection, not a general code review and not “tests passed, therefore done.”

By default, do not edit code. Diagnose and classify first.

## Inputs

Use:
- approved plan and acceptance criteria;
- implementation report;
- actual diff and current repository state;
- tests/build/runtime output;
- incumbent product/design context for UI work.

## Verification procedure

### 1. Re-ground independently

Use Graphify/source inspection to confirm affected paths and dependencies rather than trusting the executor's file list. For architecture-sensitive changes, use Archify or equivalent source-backed flow inspection to verify the intended architecture delta.

### 2. Verify every requirement

For each requirement/acceptance criterion, record:
- `PASS`, `FAIL`, or `UNPROVEN`;
- concrete evidence: test, command output, code path, runtime behavior, screenshot/render, API result, or other reproducible proof.

`UNPROVEN` is not `PASS`.

### 3. Regression verification

Check the regression surface from the plan plus any new surface revealed by the diff:
- callers and downstream consumers;
- API/schema compatibility;
- state/error/empty/loading paths;
- permissions/auth boundaries where relevant;
- existing tests around affected behavior;
- build/type integrity;
- unintended files/behaviors changed.

### 4. Diff review

Inspect for:
- changes outside the approved scope;
- duplicated abstractions or unnecessary indirection;
- hidden behavior changes;
- dead code or incomplete migrations;
- error handling that masks failures;
- tests that only mirror the implementation rather than prove behavior.

### 5. UI verification

For UI-facing changes, verification must include the **rendered result**, not source inspection alone.

Use Impeccable for deterministic/visual design checks and critique. Verify at least:
- hierarchy and readability;
- layout/alignment/spacing;
- component consistency with the product;
- loading/empty/error/disabled/selected states as relevant;
- responsive behavior at relevant widths;
- obvious accessibility issues;
- whether the implementation respects the planner's UI contract.

Use UI UX Pro Max as design reference; use Impeccable to inspect what was actually built.

Do not enter a random CSS tweak loop. Name the observed defect and its likely cause before proposing a fix.

## Outcome policy

Return exactly one primary status:

### `READY`
Every material acceptance criterion is proven and no blocking regression remains.

### `FIX_LOCAL`
A narrow defect exists, its root cause is proven, the plan remains valid, and one bounded fix should resolve it. Specify the exact defect and re-verification required.

### `REPLAN`
Use when:
- a material plan assumption is false;
- the change requires broader architecture/scope than planned;
- there is a second unexpected regression;
- repeated local fixes are accumulating;
- verification exposes missing requirements or contradictory acceptance criteria.

### `HUMAN_DECISION`
A product/architecture/risk decision is genuinely required and cannot safely be inferred from repository evidence.

## Syntaro profile

If `.agent-stack/profile` contains `syntaro`, require stronger evidence:
- explicit requirement-to-evidence mapping;
- architecture compliance for changed flows;
- missing-evidence list;
- open decision list;
- verification readiness suitable for another human or agent to audit.

Never convert uncertainty into a pass merely to complete the task.
