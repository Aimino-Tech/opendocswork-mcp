# Agent Stack v5 Minimal

## Why this shape

The stack is deliberately small. We do not maintain our own copies of debugging, regression, UI, architecture, or repo-search methodologies as separate skills. Those rules live inside the three role wrappers, while specialized capability is delegated to upstream tools.

### Owned skills

1. **planner** — evidence-first plan, architecture/UI impact, acceptance and verification contract.
2. **executor** — plan adherence, coherent implementation, root-cause-first failure handling, hard anti-loop stop.
3. **verifier** — independent requirement/regression/architecture/rendered-UI verification.

### Upstream capabilities

- **Graphify**: persistent local code graph. The official Python package is `graphifyy`; CLI command is `graphify`.
- **Archify**: source-backed architecture/workflow/data-flow diagrams and validation.
- **UI UX Pro Max**: real design datasets/templates/search tooling installed under `.agents/skills/`.
- **Impeccable**: one UI design skill, rendered/live workflows, deterministic detector suite and Codex hook.

## Target layout after installation

```text
repo/
├── AGENTS.md
├── .omp/RULES.md
├── .agents/skills/
│   ├── planner/          # ours
│   ├── executor/         # ours
│   ├── verifier/         # ours
│   ├── graphify/         # upstream
│   ├── archify/          # upstream
│   ├── ui-ux-pro-max/    # upstream
│   └── impeccable/       # upstream
├── .agent-stack/
│   ├── profile
│   ├── schemas/
│   └── chatgpt-work/
├── .codex/hooks.json     # installed by Impeccable for Codex
└── graphify-out/         # graph data
```

OMP natively discovers `.agents/skills/`, so the same skill library works for OMP and Codex without duplicate authored skills.

## Usage

You normally do not invoke every capability manually. Give the task as normal. The role wrappers decide when Graphify/Archify/UI UX Pro Max/Impeccable are needed.

Manual phase testing is still possible:
- `$planner`, `$executor`, `$verifier` in Codex-style skill invocation;
- `/skill:planner`, `/skill:executor`, `/skill:verifier` in OMP.

### UI flow

`planner -> UI UX Pro Max -> UI contract -> executor -> rendered UI -> Impeccable -> verifier`

### Anti-loop flow

First unexpected regression: reproduce and prove root cause before another edit.
Second unexpected regression: stop and replan.

## ChatGPT Work

The installer builds one ZIP per installed skill under:

`.agent-stack/chatgpt-work/upload-ready/`

ChatGPT supports uploaded Skills. Upload the skills you want in ChatGPT's Skills UI. The repository also gets `.agent-stack/chatgpt-work/PROJECT_INSTRUCTIONS.md`, which can be pasted into project instructions or used as the workflow reference when Work operates on a local folder.

## Updating upstream capabilities

Re-run the installer, or update individually:

```bash
npx -y ui-ux-pro-max-cli@latest init --ai universal --force
npx -y impeccable update
npx -y skills add tt-a1i/archify --skill archify --agent codex --copy --yes
uv tool install --upgrade graphifyy
graphify install --project --platform agents
graphify hook install
```

After an Impeccable update, Codex may ask you to approve the updated hook again.
