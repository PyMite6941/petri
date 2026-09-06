# Petri

A sandbox manager for **benign, simulated** malware specimens — small programs
you write yourself that *mimic* malware behavior, so you can break them down and
watch what they do without any of it being real. It's a malware-analysis bench
for learning how these things work by building and dissecting harmless
stand-ins.

Written in Rust, with an [egui](https://github.com/emilk/egui) desktop UI.

---

## ⚠️ This is a work in progress and it does not build yet

Read this before you clone it. Being straight about the state of the code:

- **`cargo check` currently fails.** Seven compile errors, in
  `src/sandbox/sandbox.rs` and `src/ui/ui_display.rs`. This is expected — the
  code is mid-write.
- **The sandbox does not isolate anything yet.** `PetriSandbox` is a state
  machine plus a directory path. `isolate_sandbox()` flips an enum and returns
  `Ok`; there is no namespace, no jail, no seccomp, no container behind it. The
  containment is *designed*, not *implemented*.
- **There are no specimens.** Nothing to detonate.
- **`start_sandbox()` spawns a literal placeholder** (`Command::new("some-program")`).

So: nothing here is safe to point at anything real, and there is no working
product to download and use yet. If you came looking for a finished tool, this
isn't one. It's a build log you can read.

**Never load real malware into this.** Not now, not when it compiles. See
[SECURITY.md](SECURITY.md).

---

## What it's meant to become

A desktop app where you create named sandboxes, give each one an explicit
permission set, drop a specimen into it, and watch what it does:

- **Sandboxes have a lifecycle.** `Created → Starting → Running → Stopping →
  Stopped → Destroyed`, with illegal transitions refused rather than tolerated
  (`PetriState::can_transition`).
- **Permissions are opt-in, not opt-out.** `ReadFiles`, `WriteFiles`,
  `ExecutePrograms`, `NetworkAccess` — a sandbox starts with none of them.
- **Isolation is a separate axis from run state.** `NotIsolated → Isolating →
  Isolated`, and a sandbox can only be isolated from `Running` or `Stopped`.
- **Everything lives under `sandboxes/sandbox-<id>/`** with `files/` and `logs/`
  subdirectories, created by `PetriStorage` and thrown away afterwards.

The design rule underneath all of it: a specimen never gets the real machine, it
gets a sandbox. Dangerous behaviors get *simulated and logged*, never performed —
a "C2 beacon" is recorded and never sent, "persistence" writes to a fake
registry. The point is to make intent readable, not to actually do the thing.

## Repo layout

| Path | What it is | State |
|---|---|---|
| `src/main.rs` | eframe entry point, boots `PetriApp` | works |
| `src/ui/app.rs` | the egui window — sandbox list, create/start/isolate/destroy buttons | compiles; borrow errors will surface once the lib builds |
| `src/sandbox/state.rs` | `PetriState`, `Isolated`, `PetriPermissions`, `SandboxError` | mostly there |
| `src/sandbox/sandbox.rs` | `PetriSandbox` — lifecycle, permissions, process handle | **doesn't compile**; isolation is a stub |
| `src/storage/storage.rs` | `PetriStorage` — creates the per-sandbox directory tree | works |
| `src/ui/ui_display.rs` | `Display` for `SandboxError` — the human-readable error text | declared in `mod.rs`; **doesn't compile** |
| `MILESTONES.md` | the roadmap and the accountability log | the plan |
| `run.sh`, `compile.sh` | leftovers from a Docker experiment | not wired to anything |

## Build

```bash
cargo run          # will fail until the compile errors are fixed
```

Build output goes to `dist/`, not `target/` (`.cargo/config.toml`).

Rust 2024 edition. The only direct dependency is `eframe` 0.32.

## Roadmap

[`MILESTONES.md`](MILESTONES.md) is the real plan and the honest log of what's
actually done. Short version: get it compiling, make isolation mean something,
then write the first benign specimen.

---

## License, in one paragraph

Petri is **source-available, not open source**. You can download it for free,
run it, read it, modify it privately, and quote bits of it with credit. You may
**not** redistribute it — no mirrors, no re-uploads, no package registries, no
selling it. [github.com/PyMite6941/petri](https://github.com/PyMite6941/petri)
is the only authorized place to get it. GitHub forks are fine; taking it
somewhere else is not. If you copy code out of it, credit it visibly and link
back — the exact wording is in [NOTICE](NOTICE).

Full terms: [LICENSE](LICENSE). This is a custom license, so GitHub will show it
as "Other" — don't assume MIT.

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
