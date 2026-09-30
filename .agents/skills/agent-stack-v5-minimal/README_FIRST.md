# Agent Stack v5 Minimal — start here

This is the corrected minimal version: **3 skills we own + 4 upstream capabilities**.

We own only:
- `planner`
- `executor`
- `verifier`

Installed upstream:
- Graphify — repo graph / dependency grounding
- Archify — architecture and flow modeling
- UI UX Pro Max — UI/UX design intelligence and templates
- Impeccable — rendered UI review, deterministic detectors, design workflow

Nothing else is authored as a skill. Schemas, policies, profile files and installer scripts are not skills.

## Install

From the target repository:

```bash
/path/to/agent-stack-v5-minimal/install.sh .
```

Windows PowerShell:

```powershell
& "C:\path\to\agent-stack-v5-minimal\install.ps1" -Target .
```

The installer is project-local where possible so OMP and Codex see the same `.agents/skills/` library.

## After install

For Impeccable, initialize the product/design context once inside your harness. In Codex use `$impeccable init`; in OMP invoke the Impeccable skill and run its `init` flow. In Codex, approve the project hook once via `/hooks`.

Then use your coding agent normally. For non-trivial changes, `AGENTS.md` enforces:

`planner -> executor -> verifier`

## Syntaro mode

Same skills, stricter policy:

```bash
python3 .agent-stack/bin/select_profile.py syntaro
```

Switch back:

```bash
python3 .agent-stack/bin/select_profile.py coding
```
