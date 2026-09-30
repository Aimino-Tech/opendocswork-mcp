---
name: executor
description: Execute an approved coding plan with tight scope, root-cause-first failure handling, regression limits, and plan adherence. Designed to avoid patch loops and unnecessary rewrites.
---

# Executor

Use this skill after a plan is approved. Optimize for a coherent implementation that reaches verification with as few corrective loops as possible.

## Core rule

Implement the plan; do not rediscover or redesign the task while coding. If material evidence contradicts the plan, stop and return `REPLAN` rather than improvising a new architecture inside the implementation loop.

## Before editing

1. Read the approved plan and acceptance criteria.
2. Confirm the current repo state still matches the plan.
3. Use Graphify for a scoped dependency/call-path check when useful.
4. For UI work, load the plan's UI contract and the installed UI UX Pro Max guidance. Preserve the incumbent design system.
5. Identify the smallest coherent set of files that should change.

## Implementation behavior

- Make changes in dependency order.
- Preserve existing APIs/contracts unless the plan explicitly changes them.
- Prefer existing abstractions and components over introducing parallel ones.
- Do not perform opportunistic refactors unrelated to the task.
- Do not widen scope to “clean up” neighboring code.
- Add or update tests that directly prove the planned behavior.
- For UI, implement states and responsive behavior specified by the UI contract, not just the happy-path screenshot.

## Failure protocol — anti-hamster-wheel

An unexpected failure is evidence, not an invitation to patch randomly.

### First unexpected failure/regression

Before another edit:
1. reproduce it;
2. classify whether it is caused by the current change, pre-existing, environment-related, or evidence that the plan is wrong;
3. trace the failure to a concrete cause using source/tests/runtime evidence;
4. make **one surgical fix** only if the cause is proven and local.

If the cause cannot be proven local, return `REPLAN`.

### Second unexpected regression

Stop implementation. Do not stack another speculative patch.

Return:
- what failed;
- evidence collected;
- which plan assumption appears wrong;
- files already changed;
- recommended replanning scope.

Status: `REPLAN`.

## Plan deviation rule

Any material deviation — new service, new persistence model, changed API contract, changed product behavior, broad refactor, or new UI direction — requires replanning unless the plan explicitly allowed it.

## Local verification during execution

Run focused checks as you go, but do not substitute executor self-checks for independent verification. Typical checks:
- relevant unit/integration tests;
- typecheck/build;
- lint/static analysis where meaningful;
- targeted runtime reproduction;
- UI render/smoke check for UI changes.

## Handoff report

Return a concise implementation artifact containing:
- plan/requirement IDs implemented;
- changed files;
- tests/checks run and outcomes;
- deviations from plan (preferably none);
- unexpected failures encountered and their proven causes;
- known limitations;
- evidence the verifier should inspect.

End with one of:
- `IMPLEMENTED_FOR_VERIFICATION`;
- `REPLAN`;
- `HUMAN_DECISION`.
