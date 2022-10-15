# 指数

[![badge](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fgezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com%2Fdefault%2Fsource_up_to_date%3Fowner%3Derg-lang%26repos%3Derg%26ref%3Dmain%26path%3Ddoc/EN/syntax/indexes.md%26commit_hash%3Db07c17708b9141bbce788d2e5b3ad4f365d342fa)](https://gezf7g7pd5.execute-api.ap-northeast-1.amazonaws.com/default/source_up_to_date?owner=erg-lang&repos=erg&ref=main&path=doc/EN/syntax/indexes.md&commit_hash=b07c17708b9141bbce788d2e5b3ad4f365d342fa)

有关不在此索引中的 API，请参阅 [此处](../API/index.md)

有关术语，请参见 [此处](../terms.md)

## 符号

* ! → [side&nbsp;effect](./07_side_effect.md)
  * !-type → [mutable&nbsp;type](./type/18_mut.md)
* ? → [error&nbsp;handling](./30_error_handling.md)
* &# 35; → [Str](./00_basic.md/#comment)
* $ → [shared](./type/advanced/shared.md)
* %
* &
  * &&
* [&prime;&nbsp;(single&nbsp;quote)](./20_naming_rule.md)
* [&quot;&nbsp;(double&nbsp;quote)](./01_literal.md)
* &lpar;&rpar; → [Tuple](./11_tuple.md)
* &ast;
  * &ast; → [*-less&nbsp;multiplication](./01_literal.md/#less-multiplication)
* &plus; (前置) → [operator](./06_operator.md)
  * &plus;_ → &plus; (前置)
* &plus; (中置) → [operator](./06_operator.md)
* &plus; (中置) → [Trait](./type/03_trait.md)
* ,
* &minus; (前置)
  * &minus;_ → &minus; (前置)
* &minus; (中置) → [operator](./06_operator.md)
* &minus; (中置) → [Trait](./type/03_trait.md)
  * &minus;> → [anonymous&nbsp;function](./21_lambda.md)
* . → [Visibility](./19_visibility.md)
  * [...&nbsp;assignment](./28_spread_syntax.md)
  * [...&nbsp;Extract&nbsp;assignment](./28_spread_syntax.md)
  * [...&nbsp;function](./04_function.md)
* /
* :
  * : → [Colon&nbsp;application&nbsp;style](./04_function.md)
  * : → [Declaration](./03_declaration.md)
  * : → [Keyword&nbsp;Arguments](./04_function.md)
  * :: → [visibility](./19_visibility.md)
  * := → [default&nbsp;parameters](./04_function.md)
* ;
* &lt;
  * &lt;: → [Subtype&nbsp;specification](./type/02_basic.md)
  * &lt;&lt;
  * &lt;=
* = → [Variable](./19_visibility.md)
  * ==
  * => → [procedure](./08_procedure.md)
* &gt;
  * &gt;&gt;
  * &gt;=
* @ → [decorator](./29_decorator.md)
* [] → [Array](./10_array.md)
* \ → [Indention](./00_basic.md)
* \ → [Str](./01_literal.md)
* ^
  * ^^
* _ → [Type&nbsp;erasure](./type/advanced/erasure.md)
  * &# 95;+&# 95; → &plus; (infix)
  * &# 95;-&# 95; → &minus; (infix)
* [``&nbsp;(back&nbsp;quote)](./22_subroutine.md)
* {}
  * [{} type](./type/01_type_system.md)
* {:}
* {=} → [Type&nbsp;System](./type/01_type_system.md)
  * [{=}&nbsp;type](./13_record.md)
* |
  * || → [Type variable list](./type/advanced/)
* ~

## 拉丁字母

### A

* [Add]
* [alias](type/02_basic.md)
* [Aliasing](./type/02_basic.md)
* [All&nbsp;symmetric&nbsp;types](./type/15_quantified.md)
* [algebraic&nbsp;type](./type/13_algebraic.md)
* [And]
* [and]
* [anonymous&nbsp;function](./21_lambda.md)
* [Anonymous&nbsp;polycorrelation&nbsp;coefficient](./21_lambda.md)
* anonymous type → [Type&nbsp;System](./type/01_type_system.md)
* [Array](./10_array.md)
* [assert]
* [Attach](./29_decorator.md)
* [attribute](type/09_attributive.md)
* [Attribute&nbsp;definitions](./type/02_basic.md)
* [Attribute&nbsp;Type](./type/09_attributive.md)

### B

* [Bool, Boolean](./01_literal.md)
* [Boolean&nbsp;Object](./01_literal.md)
* [borrow](./18_ownership.md)

### C

* [Cast](./type/17_type_casting.md)
* [Comments](./00_basic.md)
* [Complex&nbsp;Object](./01_literal.md)
* [Compile-time&nbsp;functions](./04_function.md)
* [circular&nbsp;references](./18_ownership.md)
* [Class](./type/04_class.md)
* [Class&nbsp;Relationship](./type/04_class.md)
* [Class&nbsp;upcasting](./type/16_subtyping.md)
* [Colon&nbsp;application&nbsp;style](./04_function.md)
* [Closure](./23_closure.md)
* [Compound Literals](./01_literal.md)
* [Complement](./type/13_algebraic.md)
* [Comprehension](./27_comprehension.md)
* [constant](./17_mutability.md)
* [Constants](./02_name.md)
* [Context](./30_error_handling.md)

### D

* [Data&nbsp;type](./type/01_type_system.md)
* [Declaration](./03_declaration.md)
* [decorator](./29_decorator.md)
* [Default&nbsp;parameters](./04_function.md)
* [Del](./02_name.md)
* [Dependent&nbsp;Type](./type/14_dependent.md)
* [Deconstructing&nbsp;a&nbsp;record](13_record.md)
* Deprecated
* [Dict](./12_dict.md)
* [Diff](./type/13_algebraic.md)
* [Difference&nbsp;from&nbsp;Data&nbsp;Class](./type/04_class.md)
* [Difference&nbsp;from&nbsp;structural&nbsp;types](type/04_class.md)
* distinct
* [Downcasting](./type/17_type_casting.md)

### E

* [Empty&nbsp;Record](./13_record.md)
* [Enum&nbsp;Class](./type/04_class.md)
* [Enum&nbsp;type](./type/11_enum.md)
* [Enumerated,&nbsp;Interval&nbsp;and&nbsp;Refinement&nbsp;Types](./type/12_refinement.md)
* [error&nbsp;handling](./30_error_handling.md)
* [Existential&nbsp;type](./type/advanced/existential.md)
* [Exponential&nbsp;Literal](./01_literal.md)
* [Extract&nbsp;assignment](./28_spread_syntax.md)

### F

* False → [Boolean Object](./01_literal.md)
* [Float&sbsp;Object](./01_literal.md)
* [for](./05_builtin_funcs.md)
* [For-All&nbsp;Patch](./type/07_patch.md)
* [freeze](./18_ownership.md)
* [Function](./04_function.md)
* [Function&nbsp;definition&nbsp;with&nbsp;multiple patterns](./04_function.md)

### G

* [GADTs(Generalized&nbsp;Algebraic&nbsp;Data&nbsp;Types)](./type/advanced/GADTs.md)
* [Generator](./34_generator.md)
* [Glue&nbsp;Patch](./type/07_patch.md)

### H

### I

* [id](./09_builtin_procs.md)
* [if](./05_builtin_funcs.md)
* [import](./33_package_system.md)
* [impl](./29_decorator.md)
* [in]
* [Indention](./00_basic.md)
* [Instant&nbsp;Block](./13_record.md)
* [Instance&nbsp;and&nbsp;class&nbsp;attributes](./type/04_class.md)
* [Implementing&nbsp;and&nbsp;resolving&nbsp;duplicate&nbsp;traits&nbsp;in&nbsp;the&nbsp;API](type/03_trait.md)
* [inheritable](./29_decorator.md)
* [inheritance](./type/05_inheritance.md)
* [Inheritance&nbsp;of&nbsp;Enumerated&nbsp;Classes](./type/05_inheritance.md)
* [Int](./01_literal.md)
* [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)
* [Interval&nbsp;Type](./type/10_interval.md)
* [Intersection](./type/13_algebraic.md)
* [Iterator](./16_iterator.md)

### J

### K

* [Keyword&nbsp;arguments](./04_function.md)
* [Kind](./type/advanced/kind.md)

### L

* lambda → [anonymous&nbsp;function](./21_lambda.md)
* let-polymorphism → [rank&nbsp;1&nbsp;polymorphism]
* [Literal&nbsp;Identifiers](./20_naming_rule.md)
* log → [side&nbsp;effect](./07_side_effect.md)

### M

* [match]
* [Marker&nbsp;Trait](./type/advanced/marker_trait.md)
* [Method](./07_side_effect.md)
* Modifier → [decorator](./29_decorator.md)
* [module](./24_module.md)
* [Multiple&nbsp;Inheritance](type/05_inheritance.md)
* [Multi-layer&nbsp;(multi-level)&nbsp;Inheritance](type/05_inheritance.md)
* [Mutable&nbsp;Type](./type/18_mut.md)
* [Mutable&nbsp;Structure&nbsp;Type](./type/advanced/mut_struct.md)
* [Mutability](./17_mutability.md)

### N

* [Nat](./01_literal.md)
* [Never]
* [New&nbsp;type](./type/advanced/newtype.md)
* [Heterogeneous&nbsp;Dict](./12_dict.md)
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
* [Overriding](./type/05_inheritance.md)
* [Override&nbsp;in&nbsp;Trait](./type/03_trait.md)

### P

* [Panic](./30_error_handling.md)
* [Patch](./type/07_patch.md)
* [Pattern&nbsp;match](./26_pattern_matching.md)
* [Phantom&nbsp;class](./type/advanced/phantom.md)
* [pipeline&nbsp;operator](./31_pipeline.md)
* [Predicate](./type/19_bound.md)
* [print!]
* [Procedures](./08_procedure.md)
* [Projection&nbsp;Type](./type/advanced/projection.md)
* Python → [Integration&nbsp;with&nbsp;Python](./32_integration_with_Python.md)

### Q

* [Quantified&nbsp;Type](./type/15_quantified.md)
* [Quantified&nbsp;Dependent&nbsp;Type](./type/advanced/quantified_dependent.md)
* [Quantified&nbsp;Types&nbsp;and&nbsp;Dependent&nbsp;Types](./type/15_quantified.md)

### R

* [Range&nbsp;Object](./01_literal.md)
* [ref]
* [ref!]
* [Record](./13_record.md)
* [Record&nbsp;Type&nbsp;Composite](./type/09_attributive.mda12_refinement.md)
* [Recursive&nbsp;functions](./04_function.md)
* [Refinement&nbsp;pattern](./type/12_refinement.md)
* [Refinement&nbsp;Type](./type/12_refinement.md)
* [replication](./18_ownership.md)
* [Replacing&nbsp;Traits](./type/05_inheritance.md)
* Result → [error&nbsp;handling](./30_error_handling.md)
* [Rewriting&nbsp;Inherited&nbsp;Attributes](./type/05_inheritance.md)
* rootobj

### S

* [Script](./00_basic.md)
* [Selecting&nbsp;Patches](./type/07_patch.md)
* self
* [Self](./type/advanced/special.md)
* [Shared&nbsp;Reference](./type/advanced/shared.md)
* [side-effect](./07_side_effect.md)
* [Smart&nbsp;Cast](./type/12_refinement.md)
* [Spread&nbsp;assignment](./28_spread_syntax.md)
* [special&nbsp;type&nbsp;variables](./type/advanced/special.md)
* [Stack&nbsp;trace](30_error_handling.md)
* [Structure&nbsp;type](./type/01_type_system.md)
* [Structural&nbsp;Patch](./type/07_patch.md)
* [Structural&nbsp;Trait](./type/03_trait.md)
* [Structural&nbsp;Subtyping](./type/01_type_system.md)
* [Structural&nbsp;types&nbsp;and&nbsp;class&nbsp;type&nbsp;relationships](./type/16_subtyping.md)
* [Str](./01_literal.md)
* [Subtyping](./type/16_subtyping.md)
* [Subtyping&nbsp;of&nbsp;subroutines](./type/16_subtyping.md)
* [Subtype&nbsp;specification](./type/02_basic.md)
* [Subtyping&nbsp;of&nbsp;Polymorphic&nbsp;Function Types](./type/15_quantified.md)
* [Subroutine&nbsp;Signatures](./22_subroutine.md)

### T

* [Test](./29_decorator.md)
* [Traits](./type/03_trait.md)
* [Trait&nbsp;inclusion](./type/03_trait.md)
* True → [Boolean&nbsp;Object](./01_literal.md)
* [True&nbsp;Algebraic&nbsp;type](./type/13_algebraic.md)
* [Type]
* [type](./15_type.md)
* [Type&nbsp;arguments&nbsp;in&nbsp;method&nbsp;definitions](./type/15_quantified.md)
* [Type&nbsp;Bound](./type/19_bound.md)
* [Type&nbsp;Definitions](./type/01_type_system.md)
* [Type&nbsp;erasure](./type/advanced/erasure.md)
* [Type&nbsp;Inference&nbsp;System](./type/01_type_system.md)
* [Type&nbsp;specification](./type/02_basic.md)
* [Type&nbsp;System](./type/01_type_system.md)
* [Type&nbsp;Widening](./type/advanced/widening.md)
* [Tuple](./11_tuple.md)

### U

* [union](type/13_algebraic.md)
* [Unit](./11_tuple.md)
* [Upcasting](type/17_type_casting.md)
* [Usage&nbsp;of&nbsp;Inheritance](./type/05_inheritance.md)

### V

* [Value&nbsp;Type](./type/08_value.md)
* [Variable](./02_name.md)
* [variable-length&nbsp;arguments](./04_function.md)

### W

* [while]

### X

### Y

### Z