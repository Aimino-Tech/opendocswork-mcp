#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, shutil
from pathlib import Path
ap=argparse.ArgumentParser(); ap.add_argument('--target',default='.'); a=ap.parse_args(); root=Path(a.target).resolve()
checks=[]
def add(name,ok,detail=''): checks.append((name,ok,detail))
for name in ('planner','executor','verifier','ui-ux-pro-max','impeccable','graphify','archify'):
    p=root/'.agents'/'skills'/name/'SKILL.md'; add(f'skill:{name}',p.exists(),str(p))
add('AGENTS.md',(root/'AGENTS.md').exists())
add('OMP sticky rules',(root/'.omp'/'RULES.md').exists())
for name in ('plan.schema.json','implementation.schema.json','verification.schema.json'):
    p=root/'.agent-stack'/'schemas'/name
    ok=False
    if p.exists():
        try: json.loads(p.read_text()); ok=True
        except Exception: pass
    add('schema:'+name,ok)
for cmd in ('node','npx','graphify'):
    add('command:'+cmd,shutil.which(cmd) is not None,shutil.which(cmd) or 'not on PATH')
for n,ok,d in checks: print(('OK  ' if ok else 'MISS'),n,('— '+d if d else ''))
missing=[x for x in checks if not x[1]]
raise SystemExit(1 if missing else 0)
