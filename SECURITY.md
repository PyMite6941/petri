# Security

Petri is an unfinished containment lab for **benign, self-written** malware
simulations. Treat everything here as experimental.

## Do not put real malware in it

Petri now runs programs inside Docker containers with the network off, a
read-only root filesystem, all capabilities dropped, and a non-root user - and
that containment has been verified. But it has been verified on **WSL2**, which
is not bare-metal Linux, and seccomp and AppArmor profiles are not written yet.
This is contained, not hardened.

It is also a personal project by someone learning Rust, not audited software.
A container is not an air-gapped VM: kernel escapes exist, and a real malware
sample deserves a disposable machine with snapshots and no network path back to
anything you care about.

Loading live malware into Petri is a breach of `LICENSE` §4 and a bad idea on
its own merits.

## Reporting a vulnerability

The bug class that matters most here is **containment escape** — anything that
lets a sandboxed process touch the host filesystem, network, registry or
processes outside its sandbox directory.

Report those **privately**, not in a public issue:

- GitHub → the repository's **Security** tab → *Report a vulnerability*
  (private vulnerability reporting is enabled), or
- open a **Discussion** marked private if that route is unavailable.

Please include what you did, what you expected, and what actually happened.

**Do not send a patch.** Petri accepts no code contributions — see
`CONTRIBUTING.md`. A clear description of the hole is worth more to me than a
fix I did not write.

## Response expectations

This is a personal project with one author and no support commitment. I will
read reports and act on containment issues, but there is no SLA, no bounty, and
no guaranteed reply.

## Supported versions

Only the current `master` of <https://github.com/PyMite6941/petri>. Copies of
Petri obtained anywhere else are unauthorized (`LICENSE` §3), unsupported, and
may have been modified.
