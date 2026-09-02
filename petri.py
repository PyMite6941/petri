#!/usr/bin/env python3
"""Petri -- benign malware-specimen lab (CLI frame).

    python petri.py list                 # list specimens
    python petri.py analyze <specimen>   # static breakdown (no execution)
    python petri.py detonate <specimen>  # run in a disposable sandbox, log it

This is the frame. The analysis engine (petri/core/analyzer.py) and the
specimens (specimens/*.py) are yours to build -- until you do, analyze/detonate
will report NotImplementedError, which is expected.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).parent
SPECIMEN_DIR = ROOT / "specimens"

SAFETY_BANNER = (
    "petri: specimens are BENIGN simulations and run only inside a disposable\n"
    "sandbox. Never load a real malware sample here -- that is not what this is."
)


def _load_specimen(name: str):
    path = SPECIMEN_DIR / f"{name}.py"
    if not path.exists():
        print(f"no such specimen: {name}")
        return None
    spec = importlib.util.spec_from_file_location(f"specimen_{name}", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def cmd_list() -> None:
    for path in sorted(SPECIMEN_DIR.glob("*.py")):
        if path.stem in ("__init__", "base"):
            continue
        mod = _load_specimen(path.stem)
        info = getattr(mod, "INFO", None)
        if info:
            print(f"  {info.name:16} [{info.family}] {info.description}")
        else:
            print(f"  {path.stem:16} (no INFO)")


def cmd_analyze(name: str) -> None:
    from petri.core.analyzer import analyze_file

    path = SPECIMEN_DIR / f"{name}.py"
    if not path.exists():
        print(f"no such specimen: {name}")
        return
    a = analyze_file(path)
    print(f"file     {a.path}")
    print(f"sha256   {a.sha256}")
    print(f"size     {a.size} bytes")
    print(f"entropy  {a.entropy} bits/byte")
    if a.dangerous_imports:
        print(f"DANGER   reaches past the sandbox: {', '.join(a.dangerous_imports)}")
    for cap, hits in a.capabilities.items():
        print(f"capability  {cap:16} <- {', '.join(hits)}")


def cmd_detonate(name: str) -> None:
    from petri.core.safety import sandbox_session

    mod = _load_specimen(name)
    if not mod:
        return
    if not hasattr(mod, "detonate"):
        print(f"{name} has no detonate()")
        return
    print(SAFETY_BANNER)
    with sandbox_session() as sb:
        mod.detonate(sb)
        print(f"\nsandbox: {sb.root}")
        print("activity:")
        for e in sb.activity:
            tag = " (simulated)" if e.simulated else ""
            print(f"  {e.kind:18} {e.target} {e.detail}{tag}")
        print("files left behind:", ", ".join(sb.list_files()) or "(none)")


def main() -> int:
    args = sys.argv[1:]
    if not args or args[0] == "list":
        cmd_list()
        return 0
    if args[0] == "analyze" and len(args) == 2:
        cmd_analyze(args[1])
        return 0
    if args[0] == "detonate" and len(args) == 2:
        cmd_detonate(args[1])
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
