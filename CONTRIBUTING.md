# Contributing

**Petri does not accept contributions. It has one author and will keep having
one author.**

That is a deliberate choice, not an oversight and not a "we'll get to it later".
Please read this before opening anything.

## Why

Petri is a personal build. The point of it is the building — working out the
sandbox model, the state machine, and the analysis engine by hand. Code that
arrives finished defeats the exercise. A pull request that solves a problem for
me takes the project away from me.

It is also a containment lab. Every line that touches isolation has to be
something I understand completely, because the whole safety argument rests on
it. I am not in a position to vouch for code I did not write.

## What that means in practice

| You want to… | Answer |
|---|---|
| Open a pull request | Will be closed unmerged, unread. Please don't. |
| Send a patch, diff or code snippet | Not accepted. Don't paste code into issues either. |
| Fix a bug you found | Tell me the bug. Don't send the fix. |
| Add a feature | No. The roadmap is `MILESTONES.md` and it is mine. |
| Become a maintainer | There are no maintainer slots. |
| Fork it for your own use | Fine — see `LICENSE` §3.1(a). Keep it a GitHub fork, keep the LICENSE and NOTICE, don't redistribute it. |
| Use the ideas in your own project | Fine, with credit. See `NOTICE`. |

Nothing you send me creates any obligation for me to use it, credit you, or
reply. There is no contributor license agreement here because there are no
contributors — see `LICENSE` §5.

## What is genuinely welcome

- **Bug reports.** "This crashes when X" / "the state machine allows an illegal
  transition" is useful. Describe the behavior, not the patch.
- **Containment holes.** If you find a way a sandboxed process reaches the host,
  that is the most valuable thing you could tell me. See `SECURITY.md` — report
  it privately, not in a public issue.
- **Questions about how it works.** Ask.

Issues may be disabled on this repository. If they are, use the contact route in
`SECURITY.md`.

## Not a support channel

This is unfinished software I work on for my own reasons. There is no support
commitment, no response-time expectation, and no roadmap negotiation. If Petri
does not do what you need, the honest answer is that it probably never will.
