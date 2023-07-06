# package manager

Erg comes standard with a package manager, which you can invoke with the `pack` subcommand.
The following are typical options.

* `erg pack init`: Initialize the current directory as a package. A `package.er` file and a `src` directory are generated. Specifying `app` will result in an executable package, `lib` will result in a library package, and `hybrid` will result in both packages. If `--license` is specified, the license file will be placed automatically.
* `erg pack build`: Build a package. With `--release` the tests are run and optimized. Artifacts are placed in `build/debug` or `build/release`.
* `erg pack install`: Install a package. In the case of libraries, `src` is placed in `.erg/lib`, and applications are placed in `.erg/app` as shell scripts. Optimize with `--release`.
* `erg pack run`: Build the package and run the application (app package only).
* `erg pack clean`: Delete the contents of the build directory.
* `erg pack test`: Run a package test. See [test.md](./test.md) for details.
* `erg pack publish`: Publish/release the package. You will need a GitHub account and public key.

This document explains how to manage your own packages.
See [install.md](./install.md) if you want to install or search for external packages.
Also see [package_system.md](../syntax/35_package_system.md) for the Erg package system.

## Standard directory structure for the whole package (for application packages)

```console
/package # package root directory
    /build # Directory to store build results
        /debug # Artifacts during debug build
        /release # Artifacts of release build
    /doc # Documents (in addition, by dividing into subdirectories such as `en`, `ja` etc., it is possible to correspond to each language)
    /src # source code
        /main.er # file that defines the main function
    /tests # Directory to store (black box) test files
    /package.er # file that defines package settings
```

## package.er

`erg pack init` will generate `package.er` file like below. `package.er` describes the configuration of the package.
Below is an example of `package.er`.

```python
name = "example" # package name
author = "John Smith" # package author name
version="0.1.0"
description = "An awesome package"
categories = ["cli"] # package categories
type = "app" # "app" or "lib"
license = "" # e.g. "MIT", "APACHE-2.0", "MIT OR Apache-2.0"
pre_build = "" # script filename to be executed before build
post_build = "" # script filename to be executed after build
dependencies = {
    # The latest one is selected if the version is not specified
    # If the version specification is omitted, the package manager automatically adds the version of the last successful build to the comments
    foo = pack("foo") # [INFO] the last successfully built version: 1.2.1
    # Packages can be renamed
    bar1 = pack("bar", "1.*.*") # [INFO] the last successfully built version: 1.2.0
    bar2 = pack("bar", "2.*.*") # [INFO] the last successfully built version: 2.0.0
    baz = pack("baz", "1.1.0")
}
deprecated=False
successors = [] # alternative packages (when a package is deprecated)
```

## Semantic versioning

Erg packages are versioned based on [semantic versioning](https://semver.org/lang/en/).
Semantic versioning is roughly specified in the format `x.y.z` (x, y, z are integers greater than or equal to 0).
The meaning of each number is as follows.

* x: major version (increase by 1 when updating breaking compatibility)
* y: minor version (increase by 1 when performing compatible updates (API addition, deprecation, etc.), bug fixes etc. are handled by patch version upgrade)
* z: patch version (increase by 1 when minor changes to fix bugs or maintain compatibility are made, serious fixes that break compatibility are handled by major version upgrades)

However, changes in version `0.*.*` are always incompatible by default. If you want to upgrade while maintaining compatibility, specify `-compatible` after it (Erg's own rule). For example, if you want to add functions while maintaining compatibility with `0.2.1`, that is, if you want to upgrade to `0.3.0`, specify `0.3.0-compatible`. Also specify `0.2.2-compatible` if you have fixed bugs.
That version will then be considered compatible with the previous version.
This works even if you want to upgrade `0.*.*` to `1.0.0`. That is, `1.0.0-compatible` is compatible with the previous version `0.y.z`.

Semantic versioning is very important when generating lockfiles. Lockfiles are files generated to keep dependencies compatible, so that newer releases of dependents depend on older packages unless explicitly updated.
Lockfiles are useful when multiple people are developing a package that has dependent packages. It also saves local storage by allowing packages that depend on them to reuse packages if they are compatible.

Erg's package manager strictly enforces these rules and will reject package updates that violate them.
The Erg package manager works with version control systems (such as git) to detect code differences and verify the correctness of versioning when a package is published.
Specifically, the package manager looks at the types of the API. A change is considered compatible if the type is a subtype of an older version (note that this is not a full verification; type-compatible but semantically-incompatible significant changes are possible, it is the developer's job to determine this).

Furthermore, since the entire package repository is registered in the registry, even developers cannot update the package without going through the package manager.
Also, packages can be deprecated but not removed.

### Appendix: Semantic Versioning Issues and Countermeasures

There are (at least) two known problems with semantic versioning.
First, semantic versioning can be too restrictive.
With semantic versioning, a single incompatible API change increments the major version of the entire package.
When this happens, things like "I wanted to try a new API, but I have to deal with another incompatible API change, so I won't upgrade".
Second, semantic versioning can promise too much.
As mentioned in the previous section, "compatible changes" to APIs are not theoretically provable. If you specify that you want a package with version `1.0.1`, you can instead use any package between `1.0.1` and `2.0.0` in terms of semantic versioning (`1.0.0` is It can't be used because a bug has been fixed), but there is a possibility that the build will not succeed due to unintended use of the API by the package developer.

Erg addresses this issue by allowing different versions of a package to be used at the same time (by renaming). This makes it possible to continue using the ver1 API while partially introducing the ver2 API.
Additionally, although it's not a very desirable state, if only a certain minor version of the API can be used without bugs, it's possible to leave it alone and move on to the next version.

## publish

Packages can be published with the `publish` subcommand. Publishing requires a GitHub account.
Packages are registered with `(owner_name)/(package_name)` by default. If you meet certain conditions (number of downloads, frequency of maintenance, etc.), you can apply to register an alias that omits the owner name.
Note that package names are case-insensitive and delimiters such as `_` and `-` are not distinguished.

Packages are stored in the registry to ensure reproducibility. Note that basically, once uploaded, the contents cannot be changed or deleted.
Updating can be accomplished only by publishing a new version.
