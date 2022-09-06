# build subcommand

The build subcommand builds the package.
The steps performed in the default build are as follows:

1. Inspect code in comments/documentation (md files under doc).
2. Compile the code needed for the package.
3. For application packages, generate batch files or shell scripts equivalent to commands.
4. Run the test.

The deliverables after the build is completed are output to the following directory.

* During debug build: build/debug
* For release build: build/release