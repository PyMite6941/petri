# Petri

A lab for **benign, simulated** malware specimens: you write small programs that
*mimic* malware behaviors, then break them down and analyze them — statically
(without running) and behaviorally (in a disposable sandbox). It's the
malware-analysis bench for learning how these things work by building and
dissecting harmless stand-ins.

## This is yours to build

The interesting parts — the analysis engine and the specimens — are **Matt's to
write**. The repo ships a *frame*, not the algorithm:

| File | State | Whose |
|---|---|---|
| `petri.py` | frame (CLI wiring) | provided |
| `petri/core/safety.py` | **implemented** — the containment guardrail | provided |
| `petri/core/analyzer.py` | **stubbed, throws** — the static breakdown engine | **you** |
| `specimens/base.py` | the specimen contract | reference |
| `specimens/*.py` | **empty** — the benign specimens | **you** |

`analyze`/`detonate` will raise `NotImplementedError` until you build them. That's
the intended starting state. Keep the frame or throw it out and design your own —
the only thing that isn't negotiable is the safety contract below.

## The one rule: containment

Everything in here is safe **only** because specimens can't touch the real
machine. The design that enforces it:

1. **A specimen never gets the real OS — it gets a `Sandbox`.** All real actions
   (file writes) are confined to a throwaway temp directory that is deleted after
   each run. Path escapes are refused.
2. **Dangerous behaviors are simulated, not performed.** "Persistence" writes to
   an in-memory fake registry; a "C2 beacon" is logged and **never sent**. Intent
   is recorded so you can analyze it; nothing leaves the sandbox.
3. **Specimens are benign by construction.** They simulate a behavior (a toy
   "locker" XORs decoy files the sandbox created); they do not do real harm.
4. **Never load a real malware sample into this.** This lab is for stand-ins you
   wrote. Analyzing live malware needs an isolated VM with no network and
   snapshots — a different setup than a Python sandbox on your daily driver.

If you replace `safety.py`, keep property (1): a specimen must not be able to
reach the host. The static analyzer flags `subprocess`/`socket`/`winreg`/`ctypes`
in a specimen precisely because those reach around the sandbox.

## Run

```bash
python petri.py list
python petri.py analyze <specimen>     # once analyzer.py is built
python petri.py detonate <specimen>    # once a specimen exists
```

Python 3.10+. No third-party dependencies.

## A suggested first milestone

See `MILESTONES.md`. The shortest path to a working loop:
1. Implement `shannon_entropy`, `extract_strings`, `analyze_file` in
   `petri/core/analyzer.py`.
2. Write one benign specimen (`specimens/toy_locker.py`) using only the sandbox.
3. `python petri.py analyze toy_locker` then `detonate toy_locker` — read your
   own specimen the way an analyst would read an unknown one.
