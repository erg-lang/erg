use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug)]
pub struct Shared<T: ?Sized>(Rc<RefCell<T>>);

impl<T: PartialEq> PartialEq for Shared<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Shared<T> {
        Self(Rc::clone(&self.0))
    }
}

impl<T: Eq> Eq for Shared<T> {}

impl<T: Hash> Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.borrow().hash(state);
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: fmt::Display> fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.borrow())
    }
}

impl<T> Shared<T> {
    pub fn new(t: T) -> Self {
        Self(Rc::new(RefCell::new(t)))
    }

    #[inline]
    pub fn into_inner(self) -> T {
        let refcell = match Rc::try_unwrap(self.0) {
            Ok(refcell) => refcell,
            Err(_rc) => panic!("unwrapping failed"),
        };
        RefCell::into_inner(refcell)
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline]
    pub fn copy(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    pub fn borrow(&self) -> Ref<'_, T> {
        RefCell::borrow(&self.0)
    }

    #[inline]
    pub fn borrow_mut(&self) -> RefMut<'_, T> {
        RefCell::borrow_mut(&self.0)
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut T {
        RefCell::as_ptr(&self.0)
    }

    /// # Safety
    /// The caller must ensure that the returned reference is not used after the underlying
    pub unsafe fn as_ref(&self) -> &T {
        self.as_ptr().as_ref().unwrap()
    }

    /// # Safety
    /// The caller must ensure that the returned reference is not used after the underlying
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn as_mut(&self) -> &mut T {
        self.as_ptr().as_mut().unwrap()
    }

    #[inline]
    pub fn try_borrow_mut(&self) -> Result<RefMut<'_, T>, std::cell::BorrowMutError> {
        RefCell::try_borrow_mut(&self.0)
    }

    pub fn can_borrow(&self) -> bool {
        RefCell::try_borrow(&self.0).is_ok()
    }

    pub fn can_borrow_mut(&self) -> bool {
        RefCell::try_borrow_mut(&self.0).is_ok()
    }
}

impl<T: Clone> Shared<T> {
    #[inline]
    pub fn clone_inner(&self) -> T {
        self.borrow().clone()
    }
}

#[derive(Debug)]
pub struct AtomicShared<T: ?Sized>(Arc<Mutex<T>>);

impl<T: PartialEq> PartialEq for AtomicShared<T>
where
    Mutex<T>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Clone for AtomicShared<T> {
    fn clone(&self) -> AtomicShared<T> {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Eq> Eq for AtomicShared<T> where Mutex<T>: Eq {}

impl<T: Hash> Hash for AtomicShared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.borrow_mut().hash(state);
    }
}

impl<T: Default> Default for AtomicShared<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: fmt::Display> fmt::Display for AtomicShared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.borrow_mut())
    }
}

impl<T> AtomicShared<T> {
    pub fn new(t: T) -> Self {
        Self(Arc::new(Mutex::new(t)))
    }

    #[inline]
    pub fn into_inner(self) -> T {
        let mutex = match Arc::try_unwrap(self.0) {
            Ok(mutex) => mutex,
            Err(_rc) => panic!("unwrapping failed"),
        };
        Mutex::into_inner(mutex).unwrap()
    }
}

impl<T: ?Sized> AtomicShared<T> {
    #[inline]
    pub fn copy(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    pub fn borrow_mut(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.0).map(|mutex| mutex.get_mut().unwrap())
    }
}

impl<T: Clone> AtomicShared<T> {
    #[inline]
    pub fn clone_inner(&self) -> T {
        self.borrow_mut().clone()
    }
}
