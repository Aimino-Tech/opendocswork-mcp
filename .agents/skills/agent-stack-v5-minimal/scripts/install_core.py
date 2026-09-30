#!/usr/bin/env python3
from __future__ import annotations
import argparse, shutil
from pathlib import Path

START='<!-- agent-stack-v5:start -->'
END='<!-- agent-stack-v5:end -->'

def merge_block(path: Path, block: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    old = path.read_text(encoding='utf-8') if path.exists() else ''
    if START in old and END in old:
        a=old.index(START); b=old.index(END)+len(END)
        new=old[:a].rstrip()+('\n\n' if old[:a].strip() else '')+block.strip()+('\n\n'+old[b:].lstrip() if old[b:].strip() else '\n')
    else:
        new=old.rstrip()+('\n\n' if old.strip() else '')+block.strip()+'\n'
    path.write_text(new, encoding='utf-8')

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('--target', default='.')
    ap.add_argument('--profile', choices=['coding','syntaro'], default='coding')
    args=ap.parse_args()
    root=Path(__file__).resolve().parents[1]
    target=Path(args.target).resolve()
    target.mkdir(parents=True, exist_ok=True)

    # Three owned skills only.
    for name in ('planner','executor','verifier'):
        src=root/'skills'/name
        dst=target/'.agents'/'skills'/name
        if dst.exists(): shutil.rmtree(dst)
        shutil.copytree(src,dst)

    # Schemas and profile.
    schema_dst=target/'.agent-stack'/'schemas'; schema_dst.mkdir(parents=True, exist_ok=True)
    for p in (root/'schemas').glob('*.json'): shutil.copy2(p,schema_dst/p.name)
    (target/'.agent-stack'/'profile').write_text(args.profile+'\n', encoding='utf-8')

    # Shared policy for Codex/OMP/other AGENTS readers.
    merge_block(target/'AGENTS.md', (root/'templates'/'AGENTS.block.md').read_text(encoding='utf-8'))

    # OMP sticky rules; do not destroy existing rules.
    omp_rules=target/'.omp'/'RULES.md'; omp_rules.parent.mkdir(parents=True, exist_ok=True)
    sticky=(root/'templates'/'OMP_RULES.md').read_text(encoding='utf-8').strip()
    existing=omp_rules.read_text(encoding='utf-8') if omp_rules.exists() else ''
    marker='# Agent Stack — sticky OMP rules'
    if marker not in existing:
        omp_rules.write_text(existing.rstrip()+('\n\n' if existing.strip() else '')+sticky+'\n', encoding='utf-8')

    # Runtime helpers and ChatGPT Work instructions.
    bindir=target/'.agent-stack'/'bin'; bindir.mkdir(parents=True, exist_ok=True)
    for helper in ('select_profile.py','build_work_bundles.py','doctor.py'):
        shutil.copy2(root/'scripts'/helper, bindir/helper)
    work=target/'.agent-stack'/'chatgpt-work'; work.mkdir(parents=True, exist_ok=True)
    shutil.copy2(root/'templates'/'WORK_PROJECT_INSTRUCTIONS.md', work/'PROJECT_INSTRUCTIONS.md')
    print(f'Core installed in {target}')
    print('Owned skills: planner, executor, verifier')
    print(f'Profile: {args.profile}')

if __name__=='__main__': main()
