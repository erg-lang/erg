use crate::hir::HIR;

pub struct HIRDesugarer {}

impl HIRDesugarer {
    pub fn desugar(hir: HIR) -> HIR {
        hir
    }

    // C = Class ...
    // C.
    //     _Self = C
    //     a = C.x
    //     x = 1
    // ↓
    // class C:
    //     def _Self(): return C
    //     def a(): return C.x()
    //     def x(): return 1
    fn _desugar_class_member(_hir: HIR) -> HIR {
        _hir
    }
}
