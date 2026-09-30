#!/usr/bin/env bash
set -euo pipefail

TARGET="${1:-.}"
PROFILE="${PROFILE:-coding}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$(cd "$TARGET" && pwd)"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Missing required command: $1" >&2; exit 1; }; }
need python3
need node
need npx

printf '\n[1/7] Installing minimal owned control plane...\n'
python3 "$ROOT/scripts/install_core.py" --target "$TARGET" --profile "$PROFILE"

cd "$TARGET"

printf '\n[2/7] Installing UI UX Pro Max into .agents/skills...\n'
npx -y ui-ux-pro-max-cli@latest init --ai universal --force

printf '\n[3/7] Installing Impeccable for Codex-compatible .agents/skills + Codex hook...\n'
npx -y impeccable install --providers=codex --scope=project

printf '\n[4/7] Installing Archify project skill...\n'
npx -y skills add tt-a1i/archify --skill archify --agent codex --copy --yes

printf '\n[5/7] Installing Graphify CLI + project Agent Skill...\n'
if command -v uv >/dev/null 2>&1; then
  uv tool install --upgrade graphifyy >/dev/null
elif command -v pipx >/dev/null 2>&1; then
  if pipx list 2>/dev/null | grep -q 'graphifyy'; then pipx upgrade graphifyy >/dev/null; else pipx install graphifyy >/dev/null; fi
else
  echo "Neither uv nor pipx found. Install one, then rerun:" >&2
  echo "  uv:   https://docs.astral.sh/uv/getting-started/installation/" >&2
  echo "  pipx: python3 -m pip install --user pipx && python3 -m pipx ensurepath" >&2
  exit 1
fi

if ! command -v graphify >/dev/null 2>&1; then
  echo "graphify was installed but is not yet on PATH. Open a new shell or run 'uv tool update-shell', then rerun install.sh." >&2
  exit 1
fi
graphify install --project --platform agents

printf '\n[6/7] Building initial repository graph and installing update hook...\n'
graphify .
graphify hook install

printf '\n[7/7] Building ChatGPT Work upload bundles and running doctor...\n'
python3 "$ROOT/scripts/build_work_bundles.py" --target "$TARGET"
python3 "$ROOT/scripts/doctor.py" --target "$TARGET" || true

cat <<'TXT'

DONE.

One-time UI setup inside your coding harness:
  OMP:   /skill:impeccable init   (or invoke the impeccable skill and ask it to init)
  Codex: $impeccable init

Codex only: open /hooks once and approve the project Impeccable hook.

Normal workflow: give the coding task normally. AGENTS.md tells the agent to use
planner -> executor -> verifier for non-trivial work.

Switch Syntaro profile:
  python3 .agent-stack/bin/select_profile.py syntaro

ChatGPT Work upload bundles:
  .agent-stack/chatgpt-work/upload-ready/
Project instructions:
  .agent-stack/chatgpt-work/PROJECT_INSTRUCTIONS.md
TXT
