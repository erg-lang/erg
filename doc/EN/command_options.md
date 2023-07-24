# Command line arguments

## subcommands

### lex

prints the result of lexical analysis.

### parse

Print parsing results.

### typecheck

Print type checking results.

### compile

Execute compilation.

### transpile

Convert to Python script.

### run (exec)

Display the result of execution.

### server

Starts the language server.

## options

### --build-features

Display features enabled at compiler build time.

### -c, --code

Specify code to execute.

### --dump-as-pyc

Output compile results as a `.pyc` file.

### -? , -h, --help

Display help.

### --mode

Specify a subcommand.

### -m, --module

Specify a module to run.

### --no-std

Compile without Erg standard library.

### -o, --opt-level

Specify the optimization level, from 0 to 3.

### --output-dir, --dest

Specify the output directory for the compiled output.

### -p, --python-version

Specify the Python version. The version number is a 32-bit unsigned integer and should be selected from [this list](https://github.com/google/pytype/blob/main/pytype/pyc/magic.py).

### --py-command, --python-command

Specifies the Python interpreter to use. Default is `python3` on Unix and `python` on Windows.

### --py-server-timeout

Specifies timeout for REPL execution. Default is 10 seconds.

### --quiet-startup, --quiet-repl

Stop displaying processor information at REPL startup.

### -t, --show-type

Show type information with REPL execution results.

### --target-version

Specify the version of the pyc file to output. The version follows semantic versioning.

### -V, --version

Display the version.

### --verbose

Controls the verbosity of the compiler output, which can be from 0 to 2.
Note that warnings cannot be turned off, even if this is set to 0.

### --

Specifies runtime arguments.
