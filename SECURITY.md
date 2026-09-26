# Security policy

CONDIMENTAL is a single-player offline game: it opens no network sockets,
runs no server, and reads only its own save and settings files under Godot's
per-user data directory. The realistic security surface is therefore small —
a crafted **save file** (RON, parsed by `sim_core`) or **settings file**, and
the project's CI and release tooling.

## Reporting a vulnerability

Please **do not open a public issue** for a security problem. Report it
privately through GitHub's
[private vulnerability reporting](https://github.com/TomWhitticase/condimental/security/advisories/new)
("Report a vulnerability" on the repository's Security tab). Include the
version or commit, the platform, and the file or steps that trigger it.

You should get an acknowledgement within a week. Fixes ship in the next
release, and the advisory is published once a fixed build is available.

## Supported versions

Only the latest release (and `main`) receives fixes. CONDIMENTAL is in alpha,
so there are no long-term-support branches.
