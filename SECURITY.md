# Security Policy

## Supported versions

s2rst is at an early `0.x` stage and is **beta — not intended for production
use** (see the [README](README.md)). There are no long-term support branches:
only the latest `0.x` release and the `master` branch receive fixes. If you
report an issue against an older version, please confirm it still reproduces on
the latest release or on `master`.

## Reporting a vulnerability

Please report security issues **privately** rather than opening a public issue
or pull request.

Preferred: use GitHub's
[private vulnerability reporting](https://github.com/torgebo/s2rst/security/advisories/new)
("Report a vulnerability" under the repository's **Security** tab). This keeps
the discussion private until a fix is available and lets us coordinate a
disclosure through a GitHub Security Advisory.

If you can't use GitHub private reporting, email **tb@starkad.no** instead.

When reporting, please include enough detail to reproduce the problem — affected
crate (`s2rst`, the Python bindings, or the wasm bindings), version or commit,
and a minimal example or test case if you can.

This is a small, best-effort project, so we can't commit to a fixed response
time, but reports will be looked at and acknowledged as soon as is practical.
Thank you for reporting responsibly.
