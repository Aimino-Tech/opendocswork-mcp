#!/usr/bin/env python3
import argparse
from pathlib import Path
p=argparse.ArgumentParser(); p.add_argument('profile', choices=['coding','syntaro']); p.add_argument('--target', default='.')
a=p.parse_args(); root=Path(a.target).resolve(); d=root/'.agent-stack'; d.mkdir(parents=True, exist_ok=True); (d/'profile').write_text(a.profile+'\n'); print(a.profile)
