
# build

`make` or `make build` for the debug binary (lands in target/

or

`make release` for the realease build

# make Cargo.toml

## no scm?

Cargo doesn't seem to have any way (even via plugin) to consider scm versioning
(e.g. git describe).

I'm very much accustomed to jenkins/github-actions specifying the version in the
environment or using python's setuptools-scm plugin to determine the version of
the build from git (or whatever)... so I find it somewhat dissapointing Cargo
can't look there -- even via plugin.

## Makefile

All cargo commands should work normally once the Cargo.toml is built from
input.toml. Simply issue `make version` or `make Cargo.toml` and everything else
will work just fine from the normal cargo commands (e.g. `cargo build
--release`).
