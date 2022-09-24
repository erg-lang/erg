# index

See [here](../API/index.md) for APIs not in this index.
See [here](../terms.md) for terminology.

## symbol

* [!](./07_side_effect.md)
  * !-type → [mutable type](./type/18_mut.md)
* [&#35;](./00_basic.md/#comment)
* [$](./type/advanced/shared.md)
* %
* &
  * &&
* [&prime;&nbsp;(single&nbsp;quote)](./20_naming_rule.md#literal-identifiers)
* [&quot;&nbsp;(double&nbsp;quote)](./01_literal.md#str-literal)
* &lpar;&rpar; → Tuple
* &ast;
  * [*-less multiplication](./01_literal.md/#less-multiplication)
* &plus; (prefix)
  * &plus;_ → &plus; (prefix)
* &plus; (infix)
* ,
* &minus; (prefix)
  * &minus;_ → &minus; (prefix)
* &minus; (infix)
  * &minus;> → anonymous&nbsp;function
* . → Visibility
* /
* :
  * :: → visibility
  * := → Default parameters
* ;
* &lt;
  * &lt;:
  * &lt;&lt;
  * &lt;=
* =
  * ==
  * => → procedure
* &gt;
  * &gt;&gt;
  * &gt;=
* ?
* @ → decorator
* [] → Array
* \
* ^
  * ^^
* _
  * &#95;+&#95; → &plus; (infix)
  * &#95;-&#95; → &minus; (infix)
* [``&nbsp;(back&nbsp;quote)](./22_subroutine.md#operator)
* {}
  * {} type → Compound literals
* {:} → Set
* {=} → Record
  * {=} type → Record
* |
  * || → Quantified Dependent Type
* ~

## alphabet

### A

* [Add](../API/types/traits/Add(R%2CO).md)
* [alias](type/02_basic.md#aliasing)
* [algebraic&nbsp;type](./type/13_algebraic.md)
* And
* and
* [anonymous&nbsp;function](./21_lambda.md)
* [Array](./10_array.md)
* assert
* [Attach](./29_decorator.md#attach)
* [attribute](type/09_attributive.md)
* [Attribute&nbsp;Type](./type/09_attributive.md)

### B

* Base
* Bool
* [borrow](18_ownership.md#borrow)

### C

* [Cast](./type/17_type_casting.md)
* [circular&nbsp;references](./18_ownership.md#circular-references)
* [Class](./type/04_class.md)
* [Closure](./23_closure.md)
* [Compound Literals](./01_literal.md#compound-literals)
* [Complement](./type/13_algebraic.md#complement)
* [Comprehension](./27_comprehension.md)
* [constant](./17_mutability.md#constant)
* [constants](./02_name.md#constants)
* [Context](./30_error_handling.md#context)

### D

* [Declaration](./03_declaration.md)
* [decorator](./29_decorator.md)
* [Default&nbsp;parameters](./04_function.md#default-parameters)
* [Dependent&nbsp;Type](./type/14_dependent.md)
* Deprecated
* [Dict](./12_dict.md)
* [Diff](./type/13_algebraic.md#diff) 
* distinct
* [Downcasting](./type/17_type_casting.md#downcasting)

### E

* [Enum&nbsp;type](./type/11_enum.md)
* [Enumerated,&nbsp;Interval&nbsp;and&nbsp;Shift&nbsp;Types](./type/12_refinement.md#enumerated-interval-and-sift-types)
* [Eq](../API/types/traits/Eq.md)
* [Erg](../faq_general.md)
* [error&nbsp;handling](./30_error_handling.md)
* [Existential&nbsp;type](./type/advanced/existential.md)
* [Extract&nbsp;assignment](./28_spread_syntax.md#extract-assignment)

### F

* [for](./05_builtin_funcs.md#for)
* [freeze](./18_ownership.md#freeze)
* [Function](./04_function.md)

### G

* [GADTs(Generalized&nbsp;Algebraic&nbsp;Data&nbsp;Types)](./type/advanced/GADTs.md)
* [Generator](./34_generator.md)
* [Glue Patch](./type/07_patch.md#glue-patch)

### H

### I

* [if](./05_builtin_funcs.md#if)
* import
* [impl](./29_decorator.md#impl)
* in
* [inheritable](./29_decorator.md#inheritable)
* [inheritance](./type/05_inheritance.md)
* [Int](./01_literal.md)
* [Interval&nbsp;Type](./type/10_interval.md)
* [Intersection](./type/13_algebraic.md#intersection)
* [Iterator](./16_iterator.md)

### J

### K

 * [Keyword&nbsp;arguments](./04_function.md#keyword-arguments)
 * [Kind](./type/advanced/kind.md)

### L

* lambda → [anonymous function]
* let-polymorphism → [rank 1 polymorphism]
* log

### M

* match
* [Marker&sbsp;Trait](./type/advanced/marker_trait.md)
* [Method](./07_side_effect.md#methods)
* Modifier → decorator
* [module](./24_module.md)
* [Mutable&nbsp;Type](./type/18_mut.md)
* [Mutable&nbsp;Structure&nbsp;Type](./type/advanced/mut_struct.md)

### N

* 
* [Nat](./01_literal.md#int-literal)
* Never
* None
* None
* Not
* not

### O

* [Object](./25_object_system.md)
* Option
* Or
* or
* [Ord](../API/types/traits/Ord.md)
* [ownership&nbsp;system](./18_ownership.md)
  * ownership

### P

* [panic](./30_error_handling.md#panic)
* [Patch](./type/07_patch.md)
* [Phantom&nbsp;class](./type/advanced/phantom.md)
* [pipeline&nbsp;operator](./31_pipeline.md)
* [print!](./../API/procs.md#print)
* [Procedures](./08_procedure.md)
* [Projection%nbsp;Type](./type/advanced/projection.md)
* [Python](../python/index.md)

### Q

* [Quantified&nbsp;Type](./type/15_quantified.md)
* [Quantified&nbsp;Dependent&nbsp;Type](./type/advanced/quantified_dependent.md)

### R

* ref
* ref!
* [Record](./13_record.md)
* [Recursive%nbsp;functions](./04_function.md#recursive-functions)
* [Refinement&nbsp;pattern](./type/12_refinement.md#refinement-pattern)
* [Refinement&nbsp;Type](./type/12_refinement.md)
* [replication](./18_ownership.md#replication)
* Result
* rootobj

### S

* [Selecting&nbsp;Patches](./type/07_patch.md#selecting-patches)
* self
* [Self](./type/special.md)
* [Shared&nbsp;Reference](./type/advanced/shared.md)
* [side-effect](./07_side_effect.md)
* [Smart&nbsp;Cast](./type/12_refinement.md#smart-cast)
* [Spread&nbsp;assignment](./28_spread_syntax.md)
* [stack&nbsp;trace](30_error_handling.md#stack-trace)
* [Str](./01_literal.md#str-literal)
* [Subtyping](./type/16_subtyping.md)
* [Subtype&nbsp;specification](./type/02_basic.md#subtype-specification)
* [Subroutine&nbsp;Signatures](./22_subroutine.md)

### T

* [Test](./29_decorator.md#test)
* [Traits](./type/03_trait.md)
* True
* [True&nbsp;Algebraic&nbsp;type](./type/13_algebraic.md#true-algebraic-type)
* [Type](./type/01_type_system.md)
* [type](./15_type.md)
* [Type erasure](./type/advanced/erasure.md)
* [Tuple](./11_tuple.md)

### U

* [union](type/13_algebraic.md#union)
* unit → Tuple
* [Upcasting](type/17_type_casting.md#upcasting)

### V

* [Value&nbsp;Type](./type/08_value.md)
* [Variable](./02_name.md)
* [variable-length&nbsp;arguments](./04_function.md#variable-length-arguments)

### W

* [while!](../API/procs.md#while-cond-bool-block---nonetype)

### X

### Y

### Z

