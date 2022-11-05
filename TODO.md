# TODOs

* [ ] Implement the specification
  * [x] Control flow
    * [x] if/if!
    * [x] match/match!
    * [x] for!
      * [x] code generation
    * [x] while!
      * [x] code generation
  * [x] operator
    * [x] + (binary/unary)
    * [x] - (binary/unary)
    * [x] *
    * [x] /
    * [x] ** (power)
    * [x] % (modulo)
    * [x] comparison
    * [x] ! (mutation)
    * [x] ..< (right-open range)
  * [ ] Pattern-matching
    * [x] Variable Pattern
    * [x] Literal Pattern
    * [x] Array Pattern
    * [x] Tuple Pattern
    * [x] Record Pattern
    * [x] Data Type Pattern
    * [ ] Refinement Pattern
  * [x] Function call
    * [x] Positional arguments
    * [x] Keyword arguments
    * [x] Variable length arguments
  * [x] Array literal
  * [x] Record literal
  * [x] Set literal
  * [x] Dict literal
  * [x] Tuple literal
  * [x] Variable visibility
  * [x] Raw identifier
  * [x] Lambda function
    * [x] lambda function with indentation
  * [x] do/do!
  * [ ] Function/Procedure definition
    * [x] Positional arguments
    * [ ] Variable length arguments
    * [x] Keyword arguments
  * [ ] Constant definition
    * [x] Simple constant definition
    * [ ] Cyclicity check
  * [ ] Method definition
    * [x] Simple method definition
    * [x] Trait implementation
  * [ ] Type definition
    * [x] Class definition
    * [x] Trait definition
    * [ ] Structural trait definition
    * [ ] Polymorphic type definition
  * [ ] Patch definition
    * [ ] Glue Patch definition
  * [ ] Range object
    * [x] Right-open range object (only for Int)
  * [ ] Decorator
  * [ ] Comprehension
    * [ ] Array
    * [ ] Dict
    * [ ] Set
    * [ ] Tuple
  * [ ] Pipeline operator
  * [ ] ? operator
  * [x] Multi-line string
  * [ ] String interpolation
  * [x] Multi-line comment
* [ ] Complete the type inference system
  * [x] Type variable
    * [x] Dependent type variable
    * [ ] Polymorphic type variable
  * [ ] Mutable type
    * [x] Dependent mutable method
  * [x] Projection type
    * [x] Projection call type
  * [x] Subtyping
    * [ ] Structural subtyping
      * [x] Refinement subtyping
    * [x] Nominal subtyping
  * [ ] Module system
    * [ ] Load Builtin Module
      * [x] `math` (partially)
      * [x] `random` (partially)
      * [x] `importlib` (partially)
      * [x] `io` (partially)
      * [x] `socket` (partially)
      * [x] `sys` (partially)
      * [x] `time` (partially)
    * [x] Load User Module
    * [ ] Recursive module
    * [x] Visibility check
  * [x] Patching
* [ ] Implement a side-effect checker
  * [x] procedure call
  * [ ] mutable type
* [x] Implement reference types (for methods)
* [ ] Implement an ownership checker
  * [x] Implement a move checker
  * [x] Implement a borrow checker
  * [ ] Implement a cycle-reference detector
* [ ] Implement a compile-time evaluator
  * [x] Builtin (Compile-time) operators
  * [ ] Compile-time operator
  * [ ] Compile-time function
* [x] Maintain unit tests
* [ ] Implement a Python parser
* [ ] Make code readable
  * [ ] Add docs comments to every functions/methods
  * [ ] Replace `Parser` (to more elegant & efficient one)
* [ ] Make error messages more readable
  * [ ] Add hints (include a URL with detailed information)
  * [x] Multiple error points indication
  * [ ] Support for languages other than English
    * [x] Japanese
    * [x] Simplified Chinese
    * [x] Traditional Chinese
* [ ] Create a playground (uses [pyodide](https://github.com/pyodide/pyodide))
* [ ] Develop the development environment
  * [x] Implement LSP (Language Server Protocol)
  * [x] Implement a syntax highlighter (REPL/debugger built-in)
  * [ ] Implement a package manager (`pack` subcommand)
  * [ ] Implement a virtual environment manager (`env` subcommand)
  * [x] Prepare an installer for each platform
  * [ ] Implement a compiling server
* [ ] Maintain documentations
  * [x] I18n
  * [ ] Write educational materials to learn Erg while creating applications (e.g. CLI chess game -> GUI chess game, calculator -> toy language)
* [ ] Develop Dyne (CPython compatible VM)
* [ ] __Undetermined__ Develop Kayser (WebAssembly backend)
* [ ] Develop Gal (LLVM backend)
