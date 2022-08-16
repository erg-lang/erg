# TODOs

* [ ] Implement the specification
  * [x] Control flow
    * [x] if/if!
    * [x] match/match!
    * [x] for!
    * [x] while!
  * [ ] operator
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
    * [ ] Tuple Pattern
    * [ ] Record Pattern
    * [ ] Data Type Pattern
    * [ ] Refinement Pattern
  * [x] Function call
    * [ ] Keyword arguments
  * [x] Array literal
  * [ ] Record literal
  * [ ] Set literal
  * [ ] Dict literal
  * [ ] Tuple literal
  * [ ] Variable visibility
  * [ ] Raw identifier
  * [x] Lambda function
    * [ ] lambda function with indentation
  * [ ] do/do!
  * [x] Function definition
  * [x] Procedure definition
  * [ ] Method definition
  * [ ] Type definition
    * [ ] Class definition
    * [ ] Trait definition
    * [ ] Structural trait definition
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
  * [ ] Multi-line string
  * [ ] String interpolation
  * [ ] Multi-line comment
* [ ] Complete the type inference system
  * [x] Type variable
    * [x] Dependent type variable
    * [ ] Polymorphic type variable
  * [ ] Mutable type
    * [x] Dependent mutable method
  * [x] Projection type
    * [ ] Polymorphic projection-type
  * [x] Subtyping
    * [x] Refinement subtyping
    * [x] Nominal subtyping
  * [ ] Module system
    * [ ] Load Builtin Module
    * [ ] Load User Module
    * [ ] Recursive module
    * [ ] Visibility check
  * [x] Patching
  * [ ] Rank-2 type
* [ ] Implement a side-effect checker
  * [x] procedure call
  * [ ] mutable type
* [x] Implement reference types (for methods)
* [ ] Implement an ownership checker
  * [x] Implement a move checker
  * [x] Implement a borrow checker
  * [ ] Implement a cycle-reference detector
* [ ] Implement a compile-time evaluator
  * [ ] Compiletime operator
  * [ ] Compile-time function
* [ ] Maintain unit tests
* [ ] Implement a Python parser
* [ ] Make code readable
  * [ ] Add docs comments to every functions
  * [ ] Replace Parser (to more elegant one)
* [ ] Make error messages more readable
  * [ ] Add hints (include a URL with detailed information)
  * [ ] Multiple error points indication
  * [ ] Support for languages other than English
    * [x] Japanese
* [ ] Create a playground (uses [pyodide](https://github.com/pyodide/pyodide))
* [ ] Develop the development environment
  * [ ] Implement LSP (Language Server Protocol)
  * [ ] Implement a syntax highlighter (REPL/debugger built-in)
  * [ ] Implement a package manager (`pack` subcommand)
  * [ ] Implement a virtual environment manager (`env` subcommand)
  * [ ] Prepare an installer for each platform
  * [ ] Implement a compiling server
* [ ] Maintain documentations
  * [ ] I18n
  * [ ] Write educational materials to learn Erg while creating applications (e.g. CLI chess game -> GUI chess game, calculator -> toy language)
* [ ] Develop Dyne (CPython compatible VM)
* [ ] Develop Kayser (WebAssembly backend)
* [ ] Develop Barye (LLVM backend)
