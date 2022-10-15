# index

See [here](../API/index.md) for APIs not in this index.

Also, see [here](../terms.md) for terminology.

## symbol

* ! → [side&nbsp;effect](./07_side_effect.md)
  * !-type → [mutable&nbsp;type](./type/18_mut.md)
* ? → [error&nbsp;handling](./30_error_handling.md)
* &# 35; → [Str](./00_basic.md/#comment)
* $ → [shared](./type/advanced/shared.md)
* %
* &
  * &&
* [&prime;&nbsp;(single&nbsp;quote)](./20_naming_rule.md# literal-identifiers)
* [&quot;&nbsp;(double&nbsp;quote)](./01_literal.md# str-literal)
* &lpar;&rpar; → [Tuple](./11_tuple.md)
* &ast;
  * &ast; → [*-less&nbsp;multiplication](./01_literal.md/#less-multiplication)
* &plus; (prefix) → [operator](./06_operator.md)
  * &plus;_ → &plus; (prefix)
* &plus; (infix) → [operator](./06_operator.md)
* &plus; (infix) → [Trait](./type/03_trait.md)
* ,
* &minus; (prefix)
  * &minus;_ → &minus; (prefix)
* &minus; (infix) → [operator](./06_operator.md)
* &minus; (infix) → [Trait](./type/03_trait.md)
  * &minus;> → [anonymous&nbsp;function](./21_lambda.md)
* . → [Visibility](./19_visibility.md)
  * .. → [closed range operator](./01_literal.md/#range-object)
  * ..< → [right-open range operator](./01_literal.md/#range-object)
  * ...
    * ... → [Extract&nbsp;assignment](./28_spread_syntax.md# extract-assignment)
    * ... → [Variable-length arguments](./04_function.md# variable-length-arguments)
* /
* :
  * : → [Colon&nbsp;application&nbsp;style](./04_function.md)
  * : → [Type ascription](./03_declaration.md)
  * : → [Keyword&nbsp;arguments](./04_function.md)
  * :: → [private variable modifier](./19_visibility.md)
  * := → [default&nbsp;parameters](./04_function.md)
* ;
* &lt;
  * &lt;: → [Subtype&nbsp;specification](./type/02_basic.md)
  * &lt;&lt;
  * &lt;=
  * &lt;.. → [left-open range operator](./01_literal.md/#range-object)
  * &lt;..&lt; → [open range operator](./01_literal.md/#range-object)
* = → [Variable](./19_visibility.md)
  * ==
  * => → [anonymous procedure operator](./08_procedure.md)
* &gt;
  * &gt;&gt;
  * &gt;=
* @ → [decorator](./29_decorator.md)
* [] → [Array](./10_array.md)
* \ → [Escaping](./00_basic.md)
* ^
  * ^^
* _ → [Type&nbsp;erasure](./type/advanced/erasure.md)
  * &# 95;+&# 95; → &plus; (infix)
  * &# 95;-&# 95; → &minus; (infix)
* [``&nbsp;(back&nbsp;quote)](./22_subroutine.md# operator)
* {}
  * [{} type](./type/01_type_system.md)
* {=} → [Type&nbsp;System](./type/01_type_system.md# classification)
  * [{=}&nbsp;type](./13_record.md# empty-record)
* |
  * || → [Type variable list](./type/advanced/)
* ~

## alphabet

### A

* [Add]
* [alias](type/02_basic.md# aliasing)
* [Aliasing](./type/02_basic.md# aliasing)
* [All&nbsp;symmetric&nbsp;types](./type/15_quantified.md# all-symmetric-types)
* [algebraic&nbsp;type](./type/13_algebraic.md)
* [And]
* [and]
* [anonymous&nbsp;function](./21_lambda.md)
* anonymous type → [Type&nbsp;system](./type/01_type_system.md)
* [Array](./10_array.md)
* [assert]
* [Attach](./29_decorator.md# attach)
* [attribute](type/09_attributive.md)
* [Attribute&nbsp;definitions](./type/02_basic.md# attribute-definitions)
* [Attribute&nbsp;type](./type/09_attributive.md)

### B

* [Bool, Boolean](./01_literal.md# boolean-object)
* [Boolean&nbsp;object](./01_literal.md# boolean-object)
* [borrowing](./18_ownership.md# borrow)

### C

* [Cast](./type/17_type_casting.md)
* [Comments](./00_basic.md# comments)
* [Complex&nbsp;object](./01_literal.md# complex-object)
* [Compile-time&nbsp;functions](./04_function.md# compile-time-functions)
* [circular&nbsp;references](./18_ownership.md# circular-references)
* [Class](./type/04_class.md)
* [Class&nbsp;relationship](./type/04_class.md# class-relationships)
* [Class&nbsp;upcasting](./type/16_subtyping.md# class-upcasting)
* [Colon&nbsp;application&nbsp;style](./04_function.md# colon-application-style)
* [Closure](./23_closure.md)
* [Compound literals](./01_literal.md# compound-literals)
* [Complement](./type/13_algebraic.md# complement)
* [Comprehension](./27_comprehension.md)
* [constant](./17_mutability.md# constant)
* [Constants](./02_name.md# constants)
* [Context](./30_error_handling.md# context)

### D

* [Data&nbsp;type](./type/01_type_system.md# data-type)
* [Declaration](./03_declaration.md)
* [decorator](./29_decorator.md)
* [Default&nbsp;parameters](./04_function.md# default-parameters)
* [Del](./02_name.md# delete-an-variable)
* [Dependent&nbsp;type](./type/14_dependent.md)
* Deprecated
* [Dict](./12_dict.md)
* [Diff](./type/13_algebraic.md# diff)
* distinct
* [Downcasting](./type/17_type_casting.md# downcasting)

### E

* [Empty&nbsp;record](./13_record.md# empty-record)
* [Enum&nbsp;class](./type/04_class.md# enum-class)
* [Enum&nbsp;type](./type/11_enum.md)
* [Enumerated,&nbsp;Interval&nbsp;and&nbsp;Refinement&nbsp;types](./type/12_refinement.md# enumerated-interval-and-refinement-types)
* [error&nbsp;handling](./30_error_handling.md)
* [Existential&nbsp;type](./type/advanced/existential.md)
* [Exponential&nbsp;literal](./01_literal.md# exponential-literal)
* [Extract&nbsp;assignment](./28_spread_syntax.md# extract-assignment)

### F

* False → [Boolean object](./01_literal.md# boolean-object)
* [Float&nbsp;object](./01_literal.md# float-object)
* [for](./05_builtin_funcs.md# for)
* [For-All&nbsp;patch](./type/07_patch.md# for-all-patch)
* [freeze](./18_ownership.md# freeze)
* [Function](./04_function.md)
* [Function&nbsp;definition&nbsp;with&nbsp;multiple patterns](./04_function.md# function-definition-with-multiple-patterns)

### G

* [GADTs(Generalized&nbsp;Algebraic&nbsp;Data&nbsp;Types)](./type/advanced/GADTs.md)
* [Generator](./34_generator.md)
* [Glue&nbsp;Patch](./type/07_patch.md# glue-patch)

### H

### I

* [id](./09_builtin_procs.md# id)
* [if](./05_builtin_funcs.md# if)
* [import](./33_package_system.md)
* [impl](./29_decorator.md# impl)
* [in]
* [Indention](./00_basic.md# indentation)
* [Instant&nbsp;block](./13_record.md# instant-block)
* [Instance/class&nbsp;attributes](./type/04_class.md# instance-and-class-attributes)
* [inheritable](./29_decorator.md# inheritable)
* [inheritance](./type/05_inheritance.md)
* [Int](./01_literal.md)
* [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)
* [Interval&nbsp;Type](./type/10_interval.md)
* [Intersection](./type/13_algebraic.md# intersection)
* [Iterator](./16_iterator.md)

### J

### K

* [Keyword&nbsp;arguments](./04_function.md# keyword-arguments)
* [Kind](./type/advanced/kind.md)

### L

* lambda → [anonymous&nbsp;function](./21_lambda.md)
* let-polymorphism → [rank&nbsp;1&nbsp;polymorphism]
* [Literal&nbsp;Identifiers](./20_naming_rule.md# literal-identifiers)

### M

* [match]
* [Marker&nbsp;trait](./type/advanced/marker_trait.md)
* [Method](./07_side_effect.md# methods)
* Modifier → [decorator](./29_decorator.md)
* [module](./24_module.md)
* [Multiple&nbsp;inheritance](type/05_inheritance.md# multiple-inheritance)
* [Multi-layer&nbsp;(multi-level)&nbsp;Inheritance](type/05_inheritance.md# multi-layer-multi-level-inheritance)
* [Mutable&nbsp;type](./type/18_mut.md)
* [Mutable&nbsp;structure&nbsp;type](./type/advanced/mut_struct.md)
* [Mutability](./17_mutability.md)

### N

* [Nat](./01_literal.md# int-literal)
* [Never]
* [New&nbsp;type](./type/advanced/newtype.md)
* [Heterogeneous&nbsp;Dict](./12_dict.md# heterogeneous-dict)
* None → [None&nbsp;Object]
* [None&nbsp;Object]
* Nominal&nbsp;Subtyping → [Class](./type/04_class.md)
* [Not]
* [not]

### O

* [Object](./25_object_system.md)
* [Option]
* [Or]
* [or]
* [Ord]
* [ownership&nbsp;system](./18_ownership.md)
* [Overloading](./type/advanced/overloading.md)
* [Overriding](./type/05_inheritance.md# overriding)
* [Override&nbsp;in&nbsp;trait](./type/03_trait.md# override-in-trait)

### P

* [Panic](./30_error_handling.md# panic)
* [Patch](./type/07_patch.md)
* [Pattern&nbsp;match](./26_pattern_matching.md)
* [Phantom&nbsp;class](./type/advanced/phantom.md)
* [pipeline&nbsp;operator](./31_pipeline.md)
* [Predicate](./type/19_bound.md# predicate)
* [print!]
* [Procedures](./08_procedure.md)
* [Projection&nbsp;type](./type/advanced/projection.md)
* Python → [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)

### Q

* [Quantified&nbsp;type](./type/15_quantified.md)
* [Quantified&nbsp;dependent&nbsp;type](./type/advanced/quantified_dependent.md)

### R

* [Range&nbsp;Object](./01_literal.md# range-object)
* [ref]
* [ref!]
* [Record](./13_record.md)
* [Recursive&nbsp;functions](./04_function.md# recursive-functions)
* [Refinement&nbsp;pattern](./type/12_refinement.md# refinement-pattern)
* [Refinement&nbsp;type](./type/12_refinement.md)
* [replication](./18_ownership.md# replication)
* [Replacing&nbsp;traits](./type/05_inheritance.md# replacing-traits-or-what-looks-like-it)
* Result → [error&nbsp;handling](./30_error_handling.md)

### S

* [Script](./00_basic.md# scripts)
* self
* [Self](./type/advanced/special.md)
* [Shared&nbsp;reference](./type/advanced/shared.md)
* [side-effect](./07_side_effect.md)
* [Smart&nbsp;cast](./type/12_refinement.md# smart-cast)
* [Spread&nbsp;assignment](./28_spread_syntax.md)
* [special&nbsp;type&nbsp;variables](./type/advanced/special.md# special-type-variables)
* [Stack&nbsp;trace](30_error_handling.md# stack-trace)
* [Structure&nbsp;type](./type/01_type_system.md# structure-type-anonymous-type)
* [Structural&nbsp;patch](./type/07_patch.md# structural-patch)
* [Structural&nbsp;trait](./type/03_trait.md# structural-traits)
* [Structural&nbsp;subtyping](./type/01_type_system.md# classification)
* [Structural&nbsp;types&nbsp;and&nbsp;class&nbsp;type&nbsp;relationships](./type/16_subtyping.md# structural-types-and-class-type-relationships)
* [Str](./01_literal.md# str-literal)
* [Subtyping](./type/16_subtyping.md)
* [Subtyping&nbsp;of&nbsp;subroutines](./type/16_subtyping.md# subtyping-of-subroutines)
* [Subtype&nbsp;specification](./type/02_basic.md# subtype-specification)
* [Subtyping&nbsp;of&nbsp;polymorphic&nbsp;function types](./type/15_quantified.md# subtyping-of-polymorphic-function-types)
* [Subroutine&nbsp;signatures](./22_subroutine.md)

### T

* [Test](./29_decorator.md# test)
* [Traits](./type/03_trait.md)
* [Trait&nbsp;inclusion](./type/03_trait.md# trait-inclusion)
* True → [Boolean&nbsp;object](./01_literal.md# boolean-object)
* [True&nbsp;algebraic&nbsp;type](./type/13_algebraic.md# true-algebraic-type)
* [Type]
* [type](./15_type.md)
* [Type&nbsp;arguments&nbsp;in&nbsp;method&nbsp;definitions](./type/15_quantified.md# type-arguments-in-method-definitions)
* [Type&nbsp;bound](./type/19_bound.md)
* [Type&nbsp;definitions](./type/01_type_system.md# type-definitions)
* [Type&nbsp;erasure](./type/advanced/erasure.md)
* [Type&nbsp;inference&nbsp;system](./type/01_type_system.md# type-inference-system)
* [Type&nbsp;specification](./type/02_basic.md# type-specification)
* [Type&nbsp;system](./type/01_type_system.md)
* [Type&nbsp;widening](./type/advanced/widening.md)
* [Tuple](./11_tuple.md)

### U

* [union](type/13_algebraic.md# union)
* [Unit](./11_tuple.md# unit)
* [Upcasting](type/17_type_casting.md# upcasting)

### V

* [Value&nbsp;type](./type/08_value.md)
* [Variable](./02_name.md)
* [variable-length&nbsp;arguments](./04_function.md# variable-length-arguments)

### W

* [while]

### X

### Y

### Z
