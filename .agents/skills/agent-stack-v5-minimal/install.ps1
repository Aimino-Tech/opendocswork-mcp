param(
  [string]$Target = ".",
  [ValidateSet("coding","syntaro")][string]$Profile = "coding"
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$Target = (Resolve-Path $Target).Path

function Need($cmd) {
  if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) { throw "Missing required command: $cmd" }
}
Need python
Need node
Need npx

Write-Host "`n[1/7] Installing minimal owned control plane..."
python "$Root\scripts\install_core.py" --target "$Target" --profile "$Profile"
Set-Location $Target

Write-Host "`n[2/7] Installing UI UX Pro Max..."
npx -y ui-ux-pro-max-cli@latest init --ai universal --force

Write-Host "`n[3/7] Installing Impeccable..."
npx -y impeccable install --providers=codex --scope=project

Write-Host "`n[4/7] Installing Archify..."
npx -y skills add tt-a1i/archify --skill archify --agent codex --copy --yes

Write-Host "`n[5/7] Installing Graphify..."
if (Get-Command uv -ErrorAction SilentlyContinue) {
  uv tool install --upgrade graphifyy | Out-Host
} elseif (Get-Command pipx -ErrorAction SilentlyContinue) {
  pipx install graphifyy | Out-Host
} else {
  throw "Install uv first (recommended: winget install astral-sh.uv) or install pipx, then rerun."
}
if (-not (Get-Command graphify -ErrorAction SilentlyContinue)) {
  throw "graphify is not on PATH yet. Open a new PowerShell (or update PATH) and rerun."
}
graphify install --project --platform agents

Write-Host "`n[6/7] Building repository graph + hook..."
graphify .
graphify hook install

Write-Host "`n[7/7] Building ChatGPT Work bundles + doctor..."
python "$Root\scripts\build_work_bundles.py" --target "$Target"
python "$Root\scripts\doctor.py" --target "$Target"

Write-Host @"

DONE.

One-time UI setup:
  OMP: invoke impeccable and run init
  Codex: `$impeccable init
Codex: open /hooks once and approve the Impeccable project hook.

Switch Syntaro profile:
  python .agent-stack/bin/select_profile.py syntaro

ChatGPT Work bundles:
  .agent-stack/chatgpt-work/upload-ready/
"@
