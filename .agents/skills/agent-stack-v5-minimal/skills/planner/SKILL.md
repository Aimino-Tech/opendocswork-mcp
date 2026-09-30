---
name: planner
description: Plan non-trivial coding work before edits. Grounds in the repository, uses architecture/UI capabilities when relevant, produces a bounded implementation and verification contract, and challenges its own assumptions before handing off.
---

# Planner

Use this skill for any non-trivial change before editing source code. The goal is first-pass correctness, not token minimization.

## Core rule

Do not implement. Build enough evidence that the executor can make one coherent change without discovering basic architecture, requirements, or UI decisions halfway through implementation.

## Capability order

Use the installed upstream capabilities rather than re-implementing them here:

1. **Graphify** for repository grounding, call paths, dependencies, and impact discovery. Prefer graph evidence before broad repetitive grep/search when a graph exists.
2. **Archify** when the change crosses boundaries, changes a runtime/data flow, or the architecture is unclear enough that a diagram/flow model reduces risk.
3. **UI UX Pro Max** for any user-facing UI/UX change. Use it before implementation to establish the design direction and constraints.
4. Existing repo evidence: source, tests, docs, ADRs, schemas, configs, issue context, and runtime behavior.

If a capability is unavailable, fall back to direct repository inspection. Never invent its output.

## Planning procedure

### 1. Restate the requested outcome

Define the user-visible or system-visible result in concrete terms. Separate requirements from implementation ideas.

### 2. Ground in the repository

Identify:
- entry points and relevant components;
- callers/callees and data/control flow;
- existing tests and behavioral contracts;
- persistence/schema/API boundaries;
- configuration and feature-flag dependencies;
- adjacent code that is likely to regress.

For a large repo, query Graphify first, then read the relevant source. Grep/search is for confirmation and details, not for reconstructing the entire architecture from scratch.

### 3. Build an impact map

State what is expected to change and what must remain unchanged. If the change spans multiple services/modules or modifies an important flow, use Archify to make the impact explicit.

### 4. Resolve UI direction before coding

For UI work, invoke UI UX Pro Max and inspect the incumbent product language. Produce a compact UI contract that covers:
- hierarchy and information architecture;
- layout/density;
- typography and spacing;
- component/state behavior;
- responsive behavior;
- accessibility constraints;
- existing design tokens/components to reuse;
- anti-patterns to avoid.

Do not let the executor invent a new visual language during implementation.

### 5. Produce the plan contract

The plan must contain:

- **Goal**
- **Requirements** with stable IDs (`R1`, `R2`, ...)
- **Evidence consulted**
- **Affected components/files**
- **Invariants / must-not-change behavior**
- **Assumptions** and how each was validated
- **Architecture impact**
- **Implementation steps** in dependency order
- **Acceptance criteria** mapped to requirement IDs
- **Verification plan** with concrete commands/runtime evidence
- **Regression surface**
- **UI contract** when applicable
- **Open decisions / missing evidence**
- **Explicit non-goals**

Keep the plan implementation-ready. Avoid generic steps such as “update backend” or “add tests.”

### 6. Adversarial review before handoff

Try to invalidate the plan:
- What existing behavior could this break?
- What dependency or call path might have been missed?
- Which assumption is weakest?
- Is the plan changing more architecture than the request requires?
- Is there a simpler implementation that preserves more existing behavior?
- Can every acceptance criterion actually be verified?

Revise the plan before handing it off if the review exposes a gap.

## Syntaro profile

If `.agent-stack/profile` contains `syntaro`, be stricter:
- surface missing evidence instead of silently filling gaps;
- distinguish implementation questions from product/CTO decisions;
- include dependencies and verification evidence required before execution;
- do not mark a plan executable while a material decision remains unresolved;
- explicitly define when the executor must stop and return for replanning.

## Handoff

End with one of:
- `PLAN_READY` — bounded and executable;
- `NEEDS_EVIDENCE` — specify exactly what evidence is missing;
- `HUMAN_DECISION` — specify the decision and why it cannot safely be inferred.

The executor receives the plan, not the planner's hidden reasoning.
