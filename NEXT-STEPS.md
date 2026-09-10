# Next steps — offline work plan

Written 2026-09-10. This is everything you need to keep going without help.

Work in **WSL Ubuntu**: `~/Documents/petri`. Verify with `cargo check`, not the
editor squiggles — rust-analyzer invents downstream errors when a type is broken.

---

## Where you are right now

`cargo check` → **7 errors**. They are all one unfinished refactor: you added
`config: SandboxConfig` and `process: PetriProcess` to `PetriSandbox`, but the
config struct was never written and the constructor still sets the old fields.

Nothing is wrong with the design. The refactor is just half-applied.

The UI (`src/ui/app.rs`) is finished and verified — it compiles cleanly the
moment the backend below exists. Don't edit it to make errors go away; the
errors are in `sandbox.rs`.

---

## Step 1 — make it compile (do this first, ~30 min)

Five changes, in this order. Re-run `cargo check` after each one.

### 1a. Define `SandboxConfig` in `src/sandbox/sandbox.rs`

Above `pub struct PetriSandbox`:

```rust
#[derive(Debug,Clone)]
pub struct SandboxConfig {
    pub name:String,
    pub program:String,
    pub network_enabled:bool,
    pub permissions:Vec<PetriPermissions>,
}
```

The UI expects these exact field names. Change them and `app.rs` needs matching
edits.

**Why a separate struct:** `Command` → `Child` is the same split in std.
`Command` is what you asked for; `Child` is the live thing. Config is *intent*
(what the user chose), the sandbox is *reality* (what is true now). Petri needs
both, because isolation makes them diverge — if a risk cuts the network, you
still need to remember the user wanted it on, so you can restore it.

### 1b. Declare the `process` module

`src/main.rs` has `mod sandbox; mod storage; mod ui;` — no `mod process;`, so
`crate::process` doesn't exist and `use crate::process::process::PetriProcess;`
fails with E0433.

Add `mod process;` to `main.rs`. Also make `lib.rs` agree — it currently lists
only `sandbox` and `ui`, so the two crate roots disagree about what the crate
contains. Either keep them in sync or delete `lib.rs` if you only ship a binary.

**This trap has bitten three times now** (`ui_display.rs`, `process.rs`,
`monitor.rs`). A `.rs` file with no `mod` line is invisible to the compiler and
gets zero checking. Habit: write the `mod` line the moment you create the file.

### 1c. `Child` needs importing in `src/process/process.rs`

```rust
use std::process::Child;
```

Currently unresolved — hidden only because the module isn't compiled yet.

### 1d. Update the constructor

```rust
pub fn new_sandbox(id:u64,config:SandboxConfig) -> Self {
    Self {
        id,
        config,
        state: PetriState::Created,
        isolate: Isolated::NotIsolated,
        directory: PathBuf::from(format!("sandboxes/sandbox-{}",id)),
        process: PetriProcess::None,
    }
}
```

It still sets `permissions: vec![]` (field is gone) and `process: None` (now an
enum, not an `Option`), and never sets `config`.

### 1e. Permissions moved into config

In `add_permissions` and `remove_permissions`, `self.permissions` becomes
`self.config.permissions` (three places).

### Also: delete the `name` field

You kept **both** `name: String` and `config: SandboxConfig`. That's two homes
for one fact, and `set_name` only updates one of them — they will drift. Delete
the field and let the getter read through:

```rust
pub fn name(&self) -> &str { &self.config.name }
```

You already have this bug elsewhere: `new_sandbox` builds the directory path,
and `PetriStorage` builds the same path independently — and they're already out
of sync (`PetriStorage` takes `u32`, ids are `u64`). One owner per fact.

---

## Step 2 — the quick win (~20 min)

### 2a. One-line fix in `state.rs`

```rust
(PetriState::Created,PetriState::Destroyed) => true,
```

Without it, Destroy fails on every freshly created sandbox with "Invalid
sandbox state transition". You'll hit this the first time you run the GUI.

### 2b. `stop_sandbox()` in `sandbox.rs`

```rust
pub fn stop_sandbox(&mut self) -> Result<(), SandboxError> {
    if !self.state.can_transition(&PetriState::Stopping) {
        return Err(SandboxError::InvalidStateTransition);
    }
    self.state = PetriState::Stopping;

    if let PetriProcess::Host(mut child) = std::mem::replace(&mut self.process,PetriProcess::None) {
        child.kill().map_err(|_| SandboxError::ProcessStopFailed)?;
        child.wait().map_err(|_| SandboxError::ProcessStopFailed)?;
    }

    self.state = PetriState::Stopped;
    Ok(())
}
```

**`std::mem::replace`** does what `Option::take()` would have done before you
switched to the enum: it moves the old value out and puts `None` in its place,
handing you ownership of the `Child`. You can't call `child.kill()` on a
borrowed field — `kill` needs to own or mutably borrow it, and the field has to
end up in a valid state either way. This is the borrow checker pushing you
toward code that says what's true: after stopping, there *is* no process.

**`kill()` then `wait()`** — `kill` sends SIGKILL, it does not reap. Skip
`wait` and you leave a zombie in the process table.

**`map_err` not `?`** — your `From<std::io::Error> for SandboxError` maps
*every* io error to `ProcessLaunchFailed`. A bare `?` here would report a failed
stop as "Failed to launch process". Name the right variant at the call site.

Then tell the UI: in `src/ui/app.rs`, replace the disabled Stop button

```rust
ui.add_enabled(false,egui::Button::new("Stop"))
    .on_disabled_hover_text("stop_sandbox() does not exist in the backend yet");
```

with

```rust
if ui.button("Stop").clicked() {
    let result = self.sandboxes[index].stop_sandbox();
    self.report(id,"stop",result);
}
```

### 2c. Give `start_sandbox` a real program

`Command::new("some-program")` fails with ENOENT every time. Use the configured
program, falling back to something harmless so you can watch a lifecycle run:

```rust
let program = if self.config.program.is_empty() { "sleep" } else { &self.config.program };
```

Once this works you can create → run → stop → destroy from the GUI and watch
the state actually change. **That's the first moment Petri feels real.** Aim for
it before you go further.

---

## Step 3 — the rest of the model

Do these in order; each unlocks the next.

**Getters.** `permissions(&self) -> &[PetriPermissions]` and
`isolation(&self) -> Isolated`. The second needs `Isolated` to derive
`Clone, Copy, PartialEq, Eq` — it only has `Debug` today, so it can't be
returned by value or compared. The UI currently reaches into the public fields
`sandbox.isolate` and `sandbox.config.permissions`; getters let it stop.

**Fix the isolation transitions.** `isolate_sandbox()` checks
`NotIsolated → Isolating`, then assigns `Isolated` without checking. And
`can_isolate` has no `Isolating → Isolated` arm, so the check would fail if you
added it. Per your own design doc that arm should exist. Same issue in
`start_sandbox`: `Starting → Running` is assigned, never validated.

**One owner for the directory.** Either `PetriStorage` computes the path and
the constructor asks it, or the reverse. Fix the `u32`/`u64` mismatch. Note you
deleted `PetriStorage::create_sandbox`, so storage currently does nothing.

**`SecurityEvent`.** Not yet — after storage.

---

## Step 4 — storage and persistence

Add `serde` + `toml` to `Cargo.toml`. Derive `Serialize, Deserialize` on
`SandboxConfig` only.

**The split, decided in advance:**

| Persisted to `config.toml` | Never persisted |
|---|---|
| id, name, program, network_enabled, permissions | state, isolation, process handle, directory |

`Child` cannot be serialized and never will be. That's not an obstacle — it's
the type system telling you runtime handles aren't configuration. Because you
put the config in its own struct, `#[derive(Serialize)]` just works; if
everything lived in `PetriSandbox` you'd be fighting `#[serde(skip)]`.

Directory is derivable from id, so don't store it.

Then: create `sandboxes/sandbox-N/{files,logs}/`, write `config.toml` on create,
append to `logs/events.log`. Destroy removes the ephemeral data but leaves logs.

When that lands, the UI's per-card log should read the backend's events instead
of the in-memory `HashMap` it uses now. Flag it and I'll switch it over.

---

## Step 5 — real isolation (the big one)

Everything above is a sandbox *manager*. This is what makes it a *sandbox*.

`isolate_sandbox()` currently flips an enum. Nothing is contained. Minimum bar
for the word to be true:

- `start_sandbox` runs `docker run -d --name petri-<id> --network none --read-only -v <dir>/files:/work ...`
- `process` becomes `PetriProcess::Container { id }` — the enum arm you already wrote
- isolate = `docker network disconnect` or `docker pause`
- destroy = `docker rm -f` plus deleting the directory

seccomp, AppArmor and fanotify sit **on top** of this. They're the difference
between contained and hardened, and each is its own project. Not a first-month
job — don't let them stall you.

**Until this ships, do not describe Petri as a secure sandbox.** The README
says so plainly and should keep saying so.

---

## Rust you'll hit, and what it's teaching

| Where | Concept |
|---|---|
| `SandboxConfig` vs `PetriSandbox` | intent vs live resource — same as `Command`/`Child`, `OpenOptions`/`File` |
| `PetriProcess` enum | one type, several shapes; adding an arm makes rustc find every place that must handle it |
| `std::mem::replace` in `stop_sandbox` | moving a value out of a struct field you only borrow |
| `map_err` vs `?` | blanket `From` conversions are convenient and lie when overused |
| `permissions() -> &[T]` | handing out a read-only view instead of the `Vec` |
| `impl Drop for PetriSandbox` | **do this in step 5** — guaranteed container cleanup, even on panic, with no `finally`. The single best argument for the language |
| Struct field vs constructor body | `Vec<PetriPermissions>` is a *type*; `vec![]` is a *value*. Two grammars |

---

## Habits worth keeping

- **`cargo check` is the authority.** The editor is a fast approximation that
  guesses wrong on broken types.
- **Fix the first error, then re-run.** Later errors are usually collateral —
  one broken type poisons every use site.
- **Write the `mod` line when you create the file.** Undeclared files get zero
  checking and silently rot.
- **When rustc names a feature you've never heard of** (like "return type
  notation"), suspect a grammar mix-up. It guesses the nearest valid syntax
  rather than reading your mind.
- **One owner per fact.** Every duplicated field is a future divergence bug.

---

## Repo

Working copy is WSL (`~/Documents/petri`); git lives in
`portfolio/petri/` on the Windows side and pushes to
`github.com/PyMite6941/petri`. Copy `src/` across, commit there.

`dist/` is the build output directory (`.cargo/config.toml`), gitignored.
`*.py` is gitignored — this is a Rust project.
