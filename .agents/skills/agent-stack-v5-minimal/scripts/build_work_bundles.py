#!/usr/bin/env python3
from __future__ import annotations
import argparse, zipfile
from pathlib import Path

def zip_skill(src: Path, dest: Path):
    with zipfile.ZipFile(dest,'w',zipfile.ZIP_DEFLATED) as z:
        for p in sorted(src.rglob('*')):
            if p.is_file(): z.write(p,p.relative_to(src))

def main():
    ap=argparse.ArgumentParser(); ap.add_argument('--target',default='.')
    a=ap.parse_args(); root=Path(a.target).resolve(); skills=root/'.agents'/'skills'; out=root/'.agent-stack'/'chatgpt-work'/'upload-ready'; out.mkdir(parents=True,exist_ok=True)
    made=[]
    if skills.exists():
        for d in sorted(skills.iterdir()):
            if d.is_dir() and (d/'SKILL.md').exists():
                dest=out/f'{d.name}.zip'; zip_skill(d,dest); made.append(dest)
    print(f'Created {len(made)} ChatGPT skill bundles in {out}')
    for p in made: print(' -',p.name)
if __name__=='__main__': main()
