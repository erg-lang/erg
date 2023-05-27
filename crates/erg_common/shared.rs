// use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::hash::{Hash, Hasher};
// use std::rc::Rc;
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::Arc;

/*#[derive(Debug)]
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
    #[track_caller]
    pub fn borrow(&self) -> Ref<'_, T> {
        RefCell::borrow(&self.0)
    }

    #[inline]
    #[track_caller]
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
}*/

#[derive(Debug)]
pub struct Shared<T: ?Sized>(Arc<RwLock<T>>);

impl<T: PartialEq> PartialEq for Shared<T>
where
    RwLock<T>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Shared<T> {
        Self(Arc::clone(&self.0))
    }
}

impl<T: Eq> Eq for Shared<T> where RwLock<T>: Eq {}

impl<T: Hash> Hash for Shared<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.borrow_mut().hash(state);
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: fmt::Display> fmt::Display for Shared<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.borrow_mut())
    }
}

impl<T> Shared<T> {
    pub fn new(t: T) -> Self {
        Self(Arc::new(RwLock::new(t)))
    }

    #[inline]
    pub fn into_inner(self) -> T {
        let mutex = match Arc::try_unwrap(self.0) {
            Ok(mutex) => mutex,
            Err(_rc) => panic!("unwrapping failed"),
        };
        RwLock::into_inner(mutex)
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline]
    pub fn copy(&self) -> Self {
        Self(self.0.clone())
    }

    #[inline]
    pub fn borrow(&self) -> RwLockReadGuard<'_, T> {
        println!("borrowing {}", std::any::type_name::<T>());
        let res = self.0.read();
        println!("borrowed successfully");
        res
    }

    #[inline]
    pub fn borrow_mut(&self) -> RwLockWriteGuard<'_, T> {
        println!("borrowing mut {}", std::any::type_name::<T>());
        let res = self.0.write();
        println!("borrowed mut successfully");
        res
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.0).map(|mutex| mutex.get_mut())
    }

    pub fn as_ptr(&self) -> *mut T {
        Arc::as_ptr(&self.0) as *mut T
    }

    pub fn can_borrow(&self) -> bool {
        self.0.try_read().is_some()
    }

    pub fn can_borrow_mut(&self) -> bool {
        self.0.try_write().is_some()
    }

    /// # Safety
    /// don't call this except you need to handle cyclic references.
    pub unsafe fn force_unlock_write(&self) {
        self.0.force_unlock_write();
    }
}

impl<T: Clone> Shared<T> {
    #[inline]
    pub fn clone_inner(&self) -> T {
        self.borrow_mut().clone()
    }
}
