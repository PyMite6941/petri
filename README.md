# Petri

A **defensive Linux sandbox manager**. Create a sandbox, choose what it's
allowed to do, run a program inside a container, and watch it. If something
looks wrong, Petri isolates the container immediately, records the event, and
lets you decide whether to release it or destroy it.

Written in Rust, with an [egui](https://github.com/emilk/egui) desktop UI and
Docker as the containment boundary. Linux first; Windows later.

---

## State of the project

**It works, and it contains.** Create → run → isolate → release → stop →
destroy all function from the GUI, backed by real containers.

Verified on WSL2 (Docker 29.1.3), with a program running inside a sandbox:

| Check | Result |
|---|---|
| Runs as | uid 1000 — not root |
| Write to its own `files/` mount | allowed, and the file appears on the host |
| Write to `/etc` | refused — read-only filesystem |
| Reach the network (when not granted) | refused — unreachable |
| See the host home directory | invisible — not in its mount namespace |

The enforcement is the Linux kernel's, not Petri's. There's no runtime check to
race or bypass, and it holds whether or not Petri is still running.

**What that claim does not cover.** This has been tested on WSL2, which is not
bare-metal Linux — container behaviour differs enough that you should verify on
real Linux before relying on it. seccomp and AppArmor profiles are not written
yet, so this is *contained*, not *hardened*. And it is a personal project by
someone learning Rust, not audited software. Don't put anything genuinely
hostile in it.

---

## How it works

**Containers are created with the sandbox, not when it runs.** `create` builds
the container; `run` starts it. So `Created` means something real on both sides.

| Petri | Docker |
|---|---|
| `create_sandbox` | `docker create` |
| `start_sandbox` | `docker start` |
| `stop_sandbox` | `docker stop -t 5` |
| `isolate_sandbox` | `docker pause` |
| `release_sandbox` | `docker unpause` |
| `destroy_sandbox` | `docker rm -f` + delete `files/` |

`docker pause` uses the freezer cgroup: every process in the container is
suspended mid-syscall. There's no window to race, which is why it's the isolate
primitive rather than disconnecting a network.

**Permissions become container flags.** `NetworkAccess` controls
`--network none`; `WriteFiles` controls `--read-only`. Always applied:
`--cap-drop ALL`, `--security-opt no-new-privileges`, `--memory 256m`,
`--pids-limit 128`, and `--user <owner of the mount>` so nothing runs as root.

**Isolation is a separate axis from the lifecycle.** `Created → Starting →
Running → Stopping → Stopped → Ran → Destroyed` is one machine;
`NotIsolated → Isolating → Isolated` is another. A risk detected while running
forces isolation without disturbing the run state, and release reverses it.

**Sandboxes survive a restart.** Config persists to `config.toml`; on launch
Petri scans `sandboxes/`, reloads each one, and rebuilds its container.
`events.log` and `security.log` live in `logs/`, which deliberately survives
destroy — the record of what a sandbox did has to outlive the sandbox.

**Containers cannot outlive Petri.** A container belongs to `dockerd`, not to
the process that made it, so `Drop` on `PetriSandbox` removes it. That runs on
normal exit, on early return, and on panic.

## Monitoring

One `docker stats` sample per second per running sandbox, turned into findings:

| Signal | Warning | Critical |
|---|---|---|
| CPU | ≥60% | ≥90% |
| Memory | ≥75% | ≥90% — about to be OOM-killed |
| Processes | ≥64 | ≥96 of the 128 limit — fork bomb |
| Network on a sandbox with none | — | **any traffic at all** |

Severity decides the response, and that rule lives on the enum rather than at
each call site:

- **Info** — recorded only
- **Warning** — logged and shown on the card, the sandbox keeps running
- **Critical** — isolated automatically

A warning deliberately doesn't stop anything. A tool that isolates on every
twitch gets ignored, and an ignored alarm is worse than none.

The network rule is the one that matters most: it isn't "too much traffic", it's
traffic that should be physically impossible. That's a containment failure, and
no amount of it is acceptable.

## Repo layout

| Path | What it is |
|---|---|
| `src/main.rs` | eframe entry point |
| `src/ui/app.rs` | the window — one card per sandbox, since sandboxes run independently |
| `src/ui/ui_display.rs` | `Display` for `SandboxError` |
| `src/sandbox/state.rs` | `PetriState`, `Isolated`, `PetriPermissions`, `SandboxError` |
| `src/sandbox/sandbox.rs` | `PetriSandbox` — lifecycle, config, persistence, risk response |
| `src/processes/processes.rs` | `PetriProcess` — the Docker layer |
| `src/processes/monitor.rs` | `Monitor`, `SecurityEvent`, `Severity` — the rules |
| `MILESTONES.md` | milestones and the dated log |

## Build and run

```bash
cargo run
```

Rust 2024, stable. Dependencies: `eframe` 0.32, `serde`, `toml`. Docker must be
installed and its daemon running — Petri warns in the header if it isn't.

**Under WSLg**, run it as:

```bash
LIBGL_ALWAYS_SOFTWARE=1 cargo run
```

WSLg advertises GPU support, so Mesa tries Zink (OpenGL on Vulkan), finds no
usable device, and the event loop dies on a broken pipe. Forcing llvmpipe skips
that path entirely; egui is light enough that software rendering is fine.

## Tests

```bash
cargo test
```

15 unit tests over the monitor rules, the stats parsers, and the shell-style
program splitter. The lifecycle has also been exercised end to end against real
Docker — create, exit detection, destroy-from-Created, config round-trip, `Drop`
removing the container, and a busy container being flagged then released.

---

## License, in one paragraph

Petri is **source-available, not open source**. You can download it for free,
run it, read it, modify it privately, and quote bits of it with credit. You may
**not** redistribute it — no mirrors, no re-uploads, no package registries, no
selling it. [github.com/PyMite6941/petri](https://github.com/PyMite6941/petri)
is the only authorized place to get it. GitHub forks are fine; taking it
somewhere else is not. If you copy code out of it, credit it visibly and link
back — the exact wording is in [NOTICE](NOTICE).

Full terms: [LICENSE](LICENSE). This is a custom license, so GitHub shows it as
"Other" — don't assume MIT.

## No contributors

**This project accepts no contributions and never will.** One author, on
purpose. Pull requests get closed unmerged, patches aren't accepted, and there
are no maintainer slots. The point of Petri is that I build it myself, and I
can't vouch for containment code I didn't write.

Bug reports are welcome — describe the behavior, not the patch. Containment
escapes go through [SECURITY.md](SECURITY.md), privately. The full policy is in
[CONTRIBUTING.md](CONTRIBUTING.md).

---

Copyright (c) 2026 PyMite6941. All rights reserved.
