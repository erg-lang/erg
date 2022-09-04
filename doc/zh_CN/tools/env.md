# env subcommand

The env subcommand specifies the erg execution environment.
Create a new execution environment with `erg env new [env name]`. An interactive tool opens, and when you specify an erg version, that version of erg will be installed (if it already exists, it will be used), and you will be able to use it as a new environment.
You can switch the environment with `erg env switch [env name]`.
The created environment can be edited with `erg env edit` to pre-install packages and specify dependencies for other languages.
The biggest feature of this command is that `erg env export` can output the information that reproduces the environment as `[env name].env.er` file. This allows you to immediately start developing in the same environment as others. Furthermore, `erg env publish` can publish the environment like a package.