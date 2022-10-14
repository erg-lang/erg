pub struct HIRDesugarer {}

impl HIRDesugarer {
    // C = Class ...
    // C.
    //     _Self = C
    //     a = C.x
    //     x = 1
    // ↓
    // class C:
    //     def _Self(): return C
    //     def a(): return C.x
    //     def x(): return 1
    fn _desugar_class_member() {}
}
