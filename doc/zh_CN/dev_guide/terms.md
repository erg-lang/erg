# Glossary

## symbol

### &excl;

A marker added to the end of an identifier to indicate that it is a procedure or variable type.
Or the mutating operator.

### [&#35;](../syntax/00_basic.md/# comment)

### $

### %

### &

### &prime; (single quote)

### &lpar;&rpar;

### &ast;

### &plus;

### &comma;

### &minus;

### ->

### &period;

### /

### &colon;

### &colon;&colon;

### &semi;

### &lt;

### &lt;&colon;

### &lt;&lt;

### &lt;=

### =

### ==

### =>

### &gt;

### &gt;&gt;

### &gt;=

### ?

### @

### []

### \

### ^

### ^^

### _

### ``

### {}

### {:}

### {=}

### |

### ||

### ~

## A

### [algebraic&nbsp;type]

### [And]

### [and]

### [assert]

### [attribute]

## B

### [Base]

### [Bool]

## C

### [Class]

## D

### Deprecated

### [distinct]

## E

### [enum&nbsp;type]

### [Eq]

### [Erg]

## F

### [for]

## G

## H

## I

### [if]

### [import]

### [in]

### [Int]

## J

## K

## L

### let-polymorphism -> [rank 1 polymorphism]

### [log]

## M

### [match]

## N

### [Nat]

### Never

### None

### [Not]

### [not]

## O

### [Option]

### [Or]

### [or]

### [Ord]

## P

### panic

### [print!](../syntax/../API/procs.md#print)

### [Python]

## Q

## R

### ref

### ref&excl;

### [Result]

### [rootobj]

## S

### self

### [Self](../syntax/type/special.md)

### [side-effect](../syntax/07_side_effect.md)

### [Str]

## T

### Traits

### [True]

### [Type]

### [type]

## U

## V

## W

### [while!]

## X

## Y

## Z

## A line

### [Assertion]

To check (typically at runtime) whether a condition is true in code. This is done using the `assert` function, etc.

``` erg
sum = !0
for! 0..10, i =>
    sum.add!i

assert sum == 55
```

### Value Object

In Erg, equivalent to base object. It can be evaluated at compile time and has a trivial comparison method.

### [Attachment patch](../syntax/29_decorator.md#attach)

A patch that gives the trait a standard implementation.

### Ad hoc polymorphism -> [No overloading](../syntax/type/overloading.md)

Polymorphism with so-called overloading.

### Attribute -> [attribute]

The `y` part in the `x.y` identifier.

### Arity

How many operands the operator takes.

### [Dependent type](../syntax/type/dependent_type.md)

A type whose argument is a value (idiomatically, not a type).

### immutable -> [immutable]

Indicates that the target will not change.
Variables in other languages ​​are also immutable/mutable, but in Erg all variables are immutable.

### arguments -> [arguments]

### instance

An object created by a class. An element of class type.

### [instant block](../syntax/00_basic.md#expression separator)

``` erg
x =
    y = f(a)
    z = g(b,c)
    y+z
```

### index

of the form `x[i]`, or the `i` part thereof. We call `x` an Indexable object.

### [indent](../syntax/00_basic.md#indent)

Align text to the right by moving toward spaces. Indentation.
Ergs represent blocks by indentation. This is called the offside rule.

### Aliases

Alias.

### error

Abnormal conditions defined in the specification.

* [Error handling]

### [operator](../syntax/06_operator.md)

An object that applies an operation to its operands. or a symbol denoting that object.

* [operator binding strength]

### Override

Overriding superclass methods in subclasses.
In Erg you have to add `Override` decorator when overriding.

### [No overloading](../syntax/type/overloading.md)

### Offside rule -> [indent](../syntax/00_basic.md#indent)

### [object]

* Object-orientation

### operand -> [operand](../syntax/06_operator.md)

### operator -> [operator](../syntax/06_operator.md)

## Ka line

### [kind](../syntax/type/advanced/kind.md)

Types of so-called types.

### [visibility]

The property of whether an identifier can be referenced externally (out of scope, or in another module or package).

### [type]

An object that groups terms.

* [type specification]
* [type erasure](../syntax/type/advanced/erasure.md)
* [type inference]
* [type annotation](../syntax/type/conv_type.md)
* [type argument]
* [type addition](../syntax/type/advanced/erasure.md)
* [type variable](../syntax/type/type_variable.md)
* [type constraint]

### [Guard]

### Encapsulation

Hiding implementation details.

### [variable]

Must not be immutable.

* [mutable object]
* [variable]
* [variable reference]
* [variable array]
* [variable arguments]

### [function](../syntax/04_function.md)

A subroutine with no side effects.

* [Functional programming](../syntax/23_scope.md#Avoiding mutable stateFunctional programming)

### base type

### nominative

Distinguish by name rather than by symmetrical structure.

* [named type] -> [class](../syntax/type/04_class.md)
* [Annunciation]
* [nominal subtype](../syntax/type/05_nst_vs_sst.md)

### capture -> [closure]

### [covariant]

In Erg, if `T <: U` then `K(T) <: K(U)` then `K` is said to be covariant.

### [keyword arguments]

`k` in the form of function call `f(k: v)`. You can specify actual arguments by formal argument name instead of by order.

### empty set -> [{}]

### section

* [Interval type](../syntax/type/11_interval.md)
* interval operator

### Embedded

Erg standard APIs not implemented in .er files.

### [class](../syntax/type/04_class.md)

Structure/abstract data type with inheritance function. In Erg, it is a type to implement named subtyping and overriding.
In Erg, modules are the responsibility of module objects, and types are the type object, while other languages ​​may be responsible for modules and types.

### [Closure]

### [global variables]

### [Clone]

### [inheritance](../syntax/type/07_inheritance.md)

To define a class that is a superset of another class.
The class that inherits is called the superclass, and the class that inherits is called the subclass.
A subclass has all the functionality of its superclass.

### high floor

* [higher-order kind](../syntax/type/advanced/kind.md)
* higher order type
* Higher-order functions

### [public variables]

### [structural subtype]

### ~~back reference~~ -> [back reference]

### [copy]

### comment

### [Collection](../syntax/10_array.md)

### Colon -> [:]

### [constructor](../syntax/type/04_class.md)

### container

### Compiler

### [compile-time computation](../syntax/04_function.md#compile-time function)

### comma -> [,]

## sa line

### recursion

Refer to yourself.

* recursive
* [Recursive function](../syntax/04_function.md#Recursive function)

### subscript -> [index]

### [subtyping polymorphism](../syntax/type/overloading.md)

Polymorphism with subtyping. Subtyping corresponds to set containment in types.

### Subroutine

An object that modularizes processing. A generic term for functions, procedures, and methods in Erg.

### [reference](../syntax/18_memory_management.md#borrowed)

* reference object
* [Reference counting (RC)](../syntax/18_memory_management.md#memory management)
* Reference equality -> [side effect](../syntax/07_side_effect.md)

### [identifier](../syntax/02_variable.md/# assignment)

### signature

* type signature

### [dict](../syntax/11_dict.md)

### [natural number] -> [Nat]

### Generics -> [Generic]

### Generator

### [projective type]

### borrow -> [reference](../syntax/18_memory_management.md#borrowed)

### [shadowing](../syntax/02_name.md# variables)

To override a reference to a variable by defining a variable with the same name in an inner scope.

### kind -> [kind](../syntax/type/advanced/kind.md)

Roughly the type of type.

### [set] -> [set]

In Erg, it means a Set object.

### Predicate

* [predicate function]

A function that returns a bool type.

### Conditional branch

### [Ownership]

The concept of object uniqueness.
If you have ownership of an object, you can take a mutable reference to it.

### Boolean -> [Bool]

### Singleton

An instance created from a class that can create only one instance. A design pattern that ensures that only one instance of a class is created.

### [Symbol] -> [Identifier](../syntax/02_name.md)

* [symbolization]

### [script](../syntax/00_basic.md# script)

A file containing an Erg program.

### Scope

Units in variable management. An outer scope cannot refer to a variable that exists in an inner scope.
Objects with a reference count of 0 are freed when the scope exits.

### spread operator -> [expansion assignment]

### [slice](../syntax/10_array.md#slice)

An object representing a subsequence of the array, generated in the form `x[a..b]`.

### control characters

### [Integer] -> [Int]

A set of natural numbers plus negative numbers.

### [set](../syntax/12_set.md)

### Semicolon -> [;]

### [Declaration](../syntax/03_declaration.md)

Explicitly type variables.

### Full name

* universal type -> [polymorphic type](../syntax/type/quantified.md)
  * closed universal
  * Open Universal
* universal function -> polycorrelation function
* universal quantification

### prefix operator

Operator `∘` applied in the form `∘x`.

### mutual recursion

### subscript -> [index]

### [attributes]

* [attribute subtype]

## Ta line

### [algebra](../syntax/02_name.md)

* [algebraic type](../syntax/type/13_algebraic.md)
* algebraic data types

### [assignment](../syntax/02_variable.md/#assignment)

### Multiple

* [Multiple inheritance](../syntax/type/07_inheritance.md/#Prohibition of multiple inheritance)
* Multiple assignment
* Overload -> [No overloading]

### Polymorphism

* [polymorphic type](../syntax/type/quantified.md)
* polycorrelation coefficient

### polymorphism -> [polymorphism]

### duck typing

### [tuple](../syntax/11_tuple.md)

### Single-phase

* Single phase
* Single-phase type
* Single correlation coefficient

### [Lazy initialization]

### Extraction Assignment

### Abstract syntax tree -> [AST]

### Infix operator

The operator `∘` applied in the form `x∘y`.

### [constant](../syntax/02_name.md/#constant)

Immutable, compile-time evaluable algebra.

* [constant type](../syntax/type/advanced/const.md)
* [constant expression](../syntax/type/advanced/const.md)

### [definition]

Allocating an object corresponding to a variable.

### Provided Attributes

Attributes available as API. Especially attributes auto-implemented by traits.

### [Apply]

To pass an argument to a function object and get the evaluation result.

### [decorator](../syntax/29_decorator.md)

``` erg
@deco
f x = ...
```

syntactic sugar, or `deco`. Roughly equal to `_f x = ...; f = deco _f`. `deco` itself is just a higher-order subroutine.

### destructor

Method called when the object is destroyed.

### procedure -> [procedure](../syntax/08_procedure.md)

A subroutine that reads and writes mutable state.
It is sometimes said that the execution result of a program can change depending on the order in which the procedures are called, but this is incorrect if we are talking about commutativity.
For example, operators that are subtypes of functions are generally not commutative.

### [default arguments](../syntax/04_function.md/#default arguments default-parameters)

A function that allows you to omit the specification of actual arguments at the time of calling by specifying default values ​​for formal arguments.

### Expand

* [expansion operator]
* [expansion assignment]

### [special format](../syntax/../API/special.md)

An object that cannot be passed as an actual argument.

### anonymous function -> [anonymous function](../syntax/20_lambda.md)

A function object created by the anonymous function operator `->`. Can be used without defining a name.

### dot operator (`.`) -> [attribute reference]

### Top

* Top type -> [Structural Object]
* Top class -> [Object]

### [trait](../syntax/type/03_trait.md)

## na line

### [Comprehension](../syntax/27_comprehension.md)

### ~~Infix operator~~ -> [Infix operator]

### [namespace]

## is a line

### [Array](../syntax/10_array.md)

### [derived type](../syntax/type/variances.md/# user-defined type variations)

### [pattern (match)](../syntax/26_pattern_matching.md)

### [package](../syntax/33_package_system.md)

### hashmap -> [dict](../syntax/11_dict.md)

### [patch](../syntax/type/07_patch.md)

### public variables -> [public variables](../syntax/19_visibility.md)

### parameter -> [argument](../syntax/04_function.md)

### [Parametric Polymorphism](../syntax/type/overloading.md)

### [contravariant](../syntax/type/advanced/variance.md)

### Compare

* [comparison operator]
* [comparable type]

### [private variable](../syntax/19_visibility.md)

### standard

* standard output
* standard input
* standard library

### [side effects](../syntax/07_side_effect.md)

Code should/not read/write external mutable state.

### complex number -> [Complex]

### [Float] -> [Float]

### private variables -> [private variables]

### Boolean algebra -> [Bool]

### [procedure](../syntax/08_procedure.md)

### [arguments](../syntax/04_function.md)

### Partial Typing -> [Subtyping]

### [immutable]

In Erg, an object should never change its contents.

* [immutable object]
* [immutable type]
* [immutable reference]

### [sieve type](../syntax/type/12_refinement.md)

### [block]

### Destructuring assignment

### [variable](../syntax/02_variable.md)

### bottom

* bottom type -> [{}]
* Bottom class -> [Never]

### [Polymorphism]

## ma line

### ~~ prefix operator ~~ -> prefix operator

### [marker type](../syntax/type/advanced/marker_trait.md)

### [anonymous function](../syntax/21_lambda.md)

### mutable -> [mutable]

### [move]

### methods

### Metacharacters

### [module](../syntax/24_module.md)

### [String] -> [Str]

* [String interpolation](../syntax/01_literal.md/#Str literal)

### Return value

## or line

### [phantom type](../syntax/type/advanced/phantom.md)

### Request Attributes

### [element]

### [call]

## Ra line

### [Library]

### lambda expression -> [anonymous function](../syntax/20_lambda.md)

### rank

* [rank2 polymorphism](../syntax/type/advanced/rank2type.md)

### [literal](../syntax/01_literal.md)

* [literal identifier](../syntax/18_naming_rule.md/#literal identifier)

### [quantified](../syntax/type/quantified.md)

### [Layout](../syntax/type/mut.md)

### [enum](../syntax/type/10_enum.md)

### [record](../syntax/12_record.md)

* [record type]
* Record Polymorphism -> [Column Polymorphism]

### [column polymorphism]

### [local variables](../syntax/19_visibility.md)

## line

### Wildcard