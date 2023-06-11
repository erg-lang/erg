use std::fmt;
use std::hash::{Hash, Hasher};
// use std::rc::Rc;
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::Arc;

#[derive(Debug)]
pub struct Shared<T: ?Sized> {
    data: Arc<RwLock<T>>,
    #[cfg(feature = "debug")]
    borrowed_at: Arc<RwLock<Option<&'static std::panic::Location<'static>>>>,
}

impl<T: PartialEq> PartialEq for Shared<T>
where
    RwLock<T>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T: ?Sized> Clone for Shared<T> {
    fn clone(&self) -> Shared<T> {
        Self {
            data: Arc::clone(&self.data),
            #[cfg(feature = "debug")]
            borrowed_at: self.borrowed_at.clone(),
        }
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
        Self {
            data: Arc::new(RwLock::new(t)),
            #[cfg(feature = "debug")]
            borrowed_at: Arc::new(RwLock::new(None)),
        }
    }

    #[inline]
    pub fn into_inner(self) -> T {
        let mutex = match Arc::try_unwrap(self.data) {
            Ok(mutex) => mutex,
            Err(_rc) => panic!("unwrapping failed"),
        };
        RwLock::into_inner(mutex)
    }
}

impl<T: ?Sized> Shared<T> {
    #[inline]
    pub fn copy(&self) -> Self {
        Self {
            data: self.data.clone(),
            #[cfg(feature = "debug")]
            borrowed_at: self.borrowed_at.clone(),
        }
    }

    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> RwLockReadGuard<'_, T> {
        #[cfg(feature = "debug")]
        {
            *self.borrowed_at.try_write().unwrap() = Some(std::panic::Location::caller());
        }
        self.data.try_read().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            {
                panic!(
                    "Shared::borrow: already borrowed at {}",
                    self.borrowed_at.read().as_ref().unwrap()
                )
            }
            #[cfg(not(feature = "debug"))]
            {
                panic!("Shared::borrow: already borrowed")
            }
        })
    }

    #[inline]
    #[track_caller]
    pub fn borrow_mut(&self) -> RwLockWriteGuard<'_, T> {
        #[cfg(feature = "debug")]
        {
            *self.borrowed_at.try_write().unwrap() = Some(std::panic::Location::caller());
        }
        self.data.try_write().unwrap_or_else(|| {
            #[cfg(feature = "debug")]
            {
                panic!(
                    "Shared::borrow_mut: already borrowed at {}",
                    self.borrowed_at.read().as_ref().unwrap()
                )
            }
            #[cfg(not(feature = "debug"))]
            {
                panic!("Shared::borrow_mut: already borrowed")
            }
        })
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.data).map(|mutex| mutex.get_mut())
    }

    pub fn as_ptr(&self) -> *mut T {
        RwLock::data_ptr(&self.data)
    }

    pub fn can_borrow(&self) -> bool {
        self.data.try_read().is_some()
    }

    pub fn can_borrow_mut(&self) -> bool {
        self.data.try_write().is_some()
    }

    /// # Safety
    /// don't call this except you need to handle cyclic references.
    pub unsafe fn force_unlock_write(&self) {
        self.data.force_unlock_write();
    }
}

impl<T: Clone> Shared<T> {
    #[inline]
    pub fn clone_inner(&self) -> T {
        self.borrow_mut().clone()
    }
}
