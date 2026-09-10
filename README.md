# Petri

A **defensive Linux sandbox manager**. Create a sandbox, choose what it's
allowed to do, run a program inside it, and watch it. If something looks wrong,
Petri isolates the sandbox immediately, records the event, and lets you decide
whether to keep it isolated or destroy it.

Written in Rust, with an [egui](https://github.com/emilk/egui) desktop UI.
Linux first; Windows later.

---

## ⚠️ This is a work in progress and it does not build yet

Read this before you clone it. Being straight about the state of the code:

- **`cargo check` currently fails.** Seven compile errors, from a half-applied
  refactor in `src/sandbox/sandbox.rs` — `SandboxConfig` is referenced but not
  yet written. The plan for fixing it is in [NEXT-STEPS.md](NEXT-STEPS.md).
- **The sandbox does not isolate anything.** `PetriSandbox` is a state machine
  plus a directory path. `isolate_sandbox()` flips an enum and returns `Ok`;
  there is no namespace, no jail, no seccomp, no container behind it. The
  containment is *designed*, not *implemented*.
- **`start_sandbox()` spawns a literal placeholder** (`Command::new("some-program")`),
  which fails with ENOENT every time.

So there is no working product here yet, and nothing in it is safe to point at
anything real. **Do not treat this as a security boundary.** If you came looking
for a finished tool, this isn't one — it's a build log you can read.

**Never load real malware into it.** Not now, not when it compiles. A userspace
Rust process on your daily driver is not a detonation environment. See
[SECURITY.md](SECURITY.md).

---

## What it's meant to become

- **Sandboxes have a lifecycle.** `Created → Starting → Running → Stopping →
  Stopped → Destroyed`, with illegal transitions refused rather than tolerated
  (`PetriState::can_transition`). `Destroyed` is terminal.
- **Isolation is a security axis, not a lifecycle state.** A risk detected while
  a sandbox is `Running` forces `NotIsolated → Isolating → Isolated`, and from
  there you either stay isolated or destroy.
- **Permissions are opt-in, not opt-out.** `ReadFiles`, `WriteFiles`,
  `ExecutePrograms`, `NetworkAccess` — a sandbox starts with none of them.
- **Config is separate from runtime.** `SandboxConfig` is what the user asked
  for and is what gets persisted; the live state, isolation status and process
  handle are not. The same split as `Command` vs `Child`.
- **Everything lives under `sandboxes/sandbox-<id>/`** with `files/` and
  `logs/`. Destroying a sandbox removes the ephemeral data; logs survive.

The rule underneath all of it: **prevention, not just detection**. The real
boundary has to be kernel- or container-enforced — Docker, seccomp, AppArmor,
fanotify permission events. A userspace scanner decides; it is never the
boundary itself. Until that lands, the word "sandbox" here is aspirational and
this README will keep saying so.

## Repo layout

| Path | What it is | State |
|---|---|---|
| `src/main.rs` | eframe entry point, boots `PetriApp` | works |
| `src/ui/app.rs` | the egui window — one card per sandbox: permissions picker, Run/Isolate/Destroy, per-card log | done |
| `src/ui/ui_display.rs` | `Display` for `SandboxError` — human-readable error text | live |
| `src/sandbox/state.rs` | `PetriState`, `Isolated`, `PetriPermissions`, `SandboxError` | mostly there |
| `src/sandbox/sandbox.rs` | `PetriSandbox` — lifecycle, permissions, process handle | **doesn't compile** — mid-refactor; isolation is a stub |
| `src/process/` | `PetriProcess` — the runtime handle (host child, or container later) | scaffolded; not declared in `main.rs` yet |
| `src/storage/storage.rs` | `PetriStorage` — the per-sandbox directory tree | stub; not called by anything |
| `NEXT-STEPS.md` | the ordered work plan, with the reasoning | current |
| `MILESTONES.md` | milestones and the dated accountability log | the plan |
| `run.sh`, `compile.sh` | leftovers from a Docker experiment | not wired to anything |

## Build

```bash
cargo run          # will fail until the compile errors are fixed
```

Rust 2024 edition, stable toolchain. The only direct dependency is `eframe` 0.32.
Build output goes to `dist/`, not `target/` (see `.cargo/config.toml`).

Developed under WSL2 + WSLg, which runs the egui window fine. Container and
kernel-policy behaviour should be checked on real Linux, since WSL2 differs.

## Roadmap

[`NEXT-STEPS.md`](NEXT-STEPS.md) is the ordered plan — what to build, in what
order, and why. [`MILESTONES.md`](MILESTONES.md) is the checklist and the honest
log of what's actually done.

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
