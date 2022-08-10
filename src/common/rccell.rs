use std::fmt;
use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
use std::hash::{Hash, Hasher};

#[derive(Debug)]
pub struct RcCell<T: ?Sized>(Rc<RefCell<T>>);

impl<T: PartialEq> PartialEq for RcCell<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Clone for RcCell<T> {
    fn clone(&self) -> RcCell<T> {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Eq> Eq for RcCell<T> {}

impl<T: Hash> Hash for RcCell<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.borrow().hash(state);
    }
}

impl<T: Default> Default for RcCell<T> {
    fn default() -> Self { Self::new(Default::default()) }
}

impl<T: fmt::Display> fmt::Display for RcCell<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.borrow())
    }
}

impl<T> RcCell<T> {
    pub fn new(t: T) -> Self { Self(Rc::new(RefCell::new(t))) }

    #[inline]
    pub fn into_inner(self) -> T {
        let refcell = match Rc::try_unwrap(self.0) {
            Ok(refcell) => refcell,
            Err(_rc) => panic!("unwrapping failed"),
        };
        RefCell::into_inner(refcell)
    }
}

impl<T: ?Sized> RcCell<T> {
    #[inline]
    pub fn copy(&self) -> Self { Self(self.0.clone()) }

    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        RefCell::borrow(&self.0)
    }

    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        RefCell::borrow_mut(&self.0)
    }
}

impl<T: Clone> RcCell<T> {
    #[inline]
    pub fn clone_inner(&self) -> T { self.borrow().clone() }
}
