# Versioning

The Erg compiler assigns version numbers according to semantic versioning.
However, during version 0, different rules than usual are applied (following more detailed rules than semantic versioning).
It is important to note that there are two types of compatibility in Erg. One is __specification compatibility__, which indicates compatibility with the language specification, and the other is __internal compatibility__, which indicates compatibility with (public) APIs such as compilers.

* During version 0, specification and internal compatibility may be broken in minor releases. This is the same as normal semantic versioning.
* During version 0, patch releases do not break specification compatibility, but there is no guarantee of internal compatibility.
* New features are mainly added in minor releases, but can also be added in patch releases if they are trivial language features or compiler features.

## Release Cycle

* Patch releases are made about once every 1~2 weeks. However, if a serious bug is discovered, it may be moved up.
* Minor releases are made about 10 times as often as patch releases, i.e., once every 3~6 months.
* Major releases are made at indefinite intervals. The schedule for version 1 release is not planned at this time.

## Nightly/Beta Releases

Erg will make nightly and beta releases on an irregular intervals. nightly releases are pre-releases of new patch releases, and beta releases are pre-releases of new minor/major releases.
Nightly and beta versions are published on crates.io, and beta versions are also published on GitHub releases.

The format of the nightly version is `0.x.y-nightly.z`. The same is true for beta versions.

Nightly releases are made almost every day (no release are made if no changes), while beta releases are made irregularly. However, once a beta release is released, a new beta release is released almost every day.
