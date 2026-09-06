# Petri — milestones & accountability log

You build the lab; I hold you to it. This file is the shared record. Each
session we work together, I'll check the boxes against what's actually in the
repo (not what you say you did) and log a dated line at the bottom.

The project moved from an early Python CLI prototype to a Rust + egui desktop
app. The milestones below are the Rust path; the old ones are dropped, since
nothing behind them was ever written. Petri is Rust only now — no Python goes
back in.

## Milestones

- [ ] **M1 — It compiles.** `cargo check` is clean. Today it fails with four
      errors:
      - `sandbox.rs:16` — `permissions: vec![]` puts a value where a type
        belongs; rustc reads it as return-type notation and throws twice.
      - `sandbox.rs:66` — `self.pernissions` is a typo for `permissions`.
      - `app.rs:45` — `PetriPermissions` has no `Display` impl but is formatted
        with `{}`.
      Expect more once those clear: `app.rs` iterates `&self.sandboxes` while
      calling `&mut self` methods on the items, and feeds a `Vec` to
      `text_edit_multiline`. Rustc stops at the first failing target.
- [ ] **M2 — The window does something.** Create a named sandbox from the UI and
      see it appear in the list. Start / isolate / destroy actually drive
      `PetriState` and the new state shows up on screen. Errors surface as text
      instead of being discarded with `let _ =`.
- [ ] **M3 — Storage is wired in.** `PetriStorage::create_sandbox` is actually
      called on create, the `sandboxes/sandbox-<id>/{files,logs}` tree appears on
      disk, and `destroy_sandbox` deletes it. Right now `PetriSandbox` invents
      the path itself and `PetriStorage` is never called (and takes a `u32`
      where the sandbox id is a `u64`).
- [ ] **M4 — Isolation means something.** `isolate_sandbox()` stops being two
      enum assignments. Minimum bar: the spawned child's working directory and
      file access are confined to the sandbox directory, path escapes are
      refused, and the permission set (`ReadFiles` / `WriteFiles` /
      `ExecutePrograms` / `NetworkAccess`) is enforced rather than just recorded.
      Until this lands, the README's containment claim is a design, not a fact.
- [ ] **M5 — First benign specimen.** A toy specimen — a "locker" that XORs
      decoy files the sandbox created — runs inside a sandbox and leaves its
      artifacts in `files/`, with the activity written to `logs/`.
- [ ] **M6 — Static breakdown.** Read a specimen without running it: entropy,
      extracted strings, and capability flags for the behaviors it simulates,
      with the matched tokens shown.
- [ ] **M7 — Static vs. dynamic.** Detonation produces a written report
      (JSON/markdown) summarizing what the specimen actually did, comparable to
      the static prediction. The agreement between the two is the payoff.
- [ ] **M8 — A specimen becomes a CTF.** Turn one specimen into a reversing
      challenge on the CTF site ("what does this do / find the marker"), closing
      the loop with the website work.

## Accountability rules

- Green boxes require **working code in the repo**, verified, not intentions.
  M1 is verified by running `cargo check`, not by saying it should build.
- If a milestone slips, that's fine — but it gets named here, not quietly
  dropped. Momentum comes from seeing the streak.
- Containment (`README.md` → the design rule, `SECURITY.md`) is checked every
  time. A specimen that reaches past the sandbox fails review regardless of how
  cool it is.
- The README must keep telling the truth about what does and doesn't work. If
  M4 isn't done, the README says isolation isn't real.

## Log

- 2026-08-25 — Frame scaffolded in the original Python prototype (containment
  guardrail + stubbed analyzer + CLI + specimen contract). All milestones open.
- 2026-09-06 — Rewritten in Rust + egui: sandbox state machine, permissions
  enum, error type, storage layout, egui window. Does not compile yet (4
  errors). Repo set up for publication: custom source-available LICENSE,
  NOTICE, CONTRIBUTING (no contributors), SECURITY, AUTHORS, README rewritten
  to match the actual code. Milestones re-cut for the Rust path. The last
  Python file was deleted and `*.py` is gitignored. Next up: M1.
