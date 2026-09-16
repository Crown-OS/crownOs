# Contributing

## Start here

```bash
git clone https://github.com/Crown-OS/crownOs && cd crownOs
./bootstrap.sh --check          # from crownOs-setup; names any missing library
cargo build --workspace
cargo test --workspace --exclude crownotify
dbus-run-session -- cargo test -p crownotify -- --test-threads=1
```

One repository, one lockfile, one toolchain. There is no workspace to assemble,
no sibling layout, and no `[patch.crates-io]` overlay — if you find instructions
mentioning any of those, they predate the merge and are wrong.

See [the README](README.md) for running it, and
[docs/architecture.md](docs/architecture.md) for how the crates fit together.

## Talk to us

[**Slack**](https://join.slack.com/t/crownosworkspace/shared_invite/zt-49zwa428j-ZfgY5WtOk4Yv2~VLuEl7mg)
— for anything quicker than an issue: a build that will not start, a question
about which crate something belongs in, or telling us the setup instructions did
not work on your distribution. That last one is genuinely useful; CI covers Arch,
Fedora and Debian, and everything else is unverified.

For anything worth finding again later — a bug, a proposal, a decision — open an
issue instead. Chat is not searchable six months from now.

## What is worth knowing before you change something

**The config schema is a contract.** `crownos-config` is read by the compositor,
the bar and the dictation daemon. Changing a section changes all of them, and the
only thing that catches a mistake is that they compile together. They did not
always live in one repository, and a schema change broke the compositor for eight
days without anything noticing — this repository exists partly so that cannot
happen again.

**Only crownshell talks Wayland client protocol.** If your component needs to
reach `smithay-client-toolkit` directly, the honest fix is a new crownshell API,
not an import. Two components currently break this rule and both are waiting on
the same missing method.

**A config file that does not parse silently reverts its whole section to
defaults.** No error, anywhere. If you touch parsing, keep that in mind — and if
you can improve it, that is a genuinely valuable change.

## Tests

`cargo test --workspace` runs 263 of them. crownotify is excluded from that run
and given its own, because its tests register real well-known names on the
session bus and cannot run concurrently.

New behaviour needs a test. Bug fixes need a test that fails before the fix — it
is the only way to know the fix does anything. The compositor's layout and
shortcut code is well covered and is a good model to imitate.

## Style

`cargo fmt --all` before every commit. **rustfmt blocks CI**, clippy does not —
roughly 15,000 lines have never been linted, so `-D warnings` today would make
every pull request red for reasons unrelated to it. Do not add to the backlog,
and clearing part of it is welcome work.

The toolchain is pinned to 1.88.0 in `rust-toolchain.toml`. rustup honours that
over whatever you have installed, so everyone compiles with the same rustc. The
pin is set by the dependency graph — vello 0.9 and xilem 0.4 both declare 1.88 —
not by the edition.

Comments should say *why*, not *what*. The code already says what it does. A
comment that explains a decision, a constraint, or something surprising is worth
writing; one that narrates the next line is not.

## Commits

Conventional Commits, with the crate as the scope:

```
fix(crownbar): read bar_height from the appearance section

The bar hardcoded 40 while the schema said 32, so the setting did nothing.
```

Scopes: `crownpositor`, `crownshell`, `crownos-config`, `crownbar`, `crowndock`,
`crownotify`, `crowndictator`, `crownuikit`, `ci`, `docs`.

Types: `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, `ci`, `chore`.

One logical change per commit. A formatting sweep and a behaviour change in the
same commit make both impossible to review.

## Pull requests

Branch from `main`, rebase onto it before opening, and expect CI to run the full
workspace — path filtering is deliberately not used, because it would hide the
cross-crate breakage this repository exists to catch.

If your change touches the config schema, say so in the description and name the
consumers you checked. That is the change most likely to break something at a
distance.

## Where the rest of CrownOS lives

The nine Rust crates are here. These are separate because they are not Rust, or
not part of the desktop:

[crownOs-setup](https://github.com/Crown-OS/crownOs-setup) ·
[crownos-iso](https://github.com/Crown-OS/crownos-iso) ·
[crownos-website](https://github.com/Crown-OS/crownos-website) ·
[crowncrate-linux](https://github.com/Crown-OS/crowncrate-linux) ·
[crowncrate-android](https://github.com/Crown-OS/crowncrate-android) ·
[lls-protocol](https://github.com/Crown-OS/lls-protocol)

## Conduct

Be decent. Problems go to the maintainers, privately if you prefer:
[CODE_OF_CONDUCT.md](https://github.com/Crown-OS/.github/blob/main/CODE_OF_CONDUCT.md).
Security issues go through
[SECURITY.md](https://github.com/Crown-OS/.github/blob/main/SECURITY.md), not a
public issue.
