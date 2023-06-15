use std::fmt;
use std::hash::{Hash, Hasher};
// use std::rc::Rc;
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::sync::Arc;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
pub struct BorrowInfo {
    location: Option<&'static std::panic::Location<'static>>,
    thread_name: String,
}

impl std::fmt::Display for BorrowInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.location {
            Some(location) => write!(
                f,
                "{}:{}, thread: {}",
                location.file(),
                location.line(),
                self.thread_name
            ),
            None => write!(f, "unknown, thread: {}", self.thread_name),
        }
    }
}

impl BorrowInfo {
    pub fn new(location: Option<&'static std::panic::Location<'static>>) -> Self {
        Self {
            location,
            thread_name: std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
        }
    }
}

#[derive(Debug)]
pub struct Shared<T: ?Sized> {
    data: Arc<RwLock<T>>,
    #[cfg(any(debug_assertions, feature = "debug"))]
    last_borrowed_at: Arc<RwLock<BorrowInfo>>,
    #[cfg(any(debug_assertions, feature = "debug"))]
    last_mut_borrowed_at: Arc<RwLock<BorrowInfo>>,
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
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_borrowed_at: self.last_borrowed_at.clone(),
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_mut_borrowed_at: self.last_mut_borrowed_at.clone(),
        }
    }
}

impl<T: Eq> Eq for Shared<T> where RwLock<T>: Eq {}

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
        Self {
            data: Arc::new(RwLock::new(t)),
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_borrowed_at: Arc::new(RwLock::new(BorrowInfo::new(None))),
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_mut_borrowed_at: Arc::new(RwLock::new(BorrowInfo::new(None))),
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
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_borrowed_at: self.last_borrowed_at.clone(),
            #[cfg(any(debug_assertions, feature = "debug"))]
            last_mut_borrowed_at: self.last_mut_borrowed_at.clone(),
        }
    }

    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> RwLockReadGuard<'_, T> {
        #[cfg(any(debug_assertions, feature = "debug"))]
        {
            *self.last_borrowed_at.try_write_for(TIMEOUT).unwrap() =
                BorrowInfo::new(Some(std::panic::Location::caller()));
        }
        self.data.try_read_for(TIMEOUT).unwrap_or_else(|| {
            #[cfg(any(debug_assertions, feature = "debug"))]
            {
                panic!(
                    "Shared::borrow: already borrowed at {}, mutably borrowed at {:?}",
                    self.last_borrowed_at.try_read_for(TIMEOUT).unwrap(),
                    self.last_mut_borrowed_at.try_read_for(TIMEOUT).unwrap()
                )
            }
            #[cfg(not(any(debug_assertions, feature = "debug")))]
            {
                panic!("Shared::borrow: already borrowed")
            }
        })
    }

    #[inline]
    #[track_caller]
    pub fn borrow_mut(&self) -> RwLockWriteGuard<'_, T> {
        #[cfg(any(debug_assertions, feature = "debug"))]
        {
            let caller = std::panic::Location::caller();
            *self.last_borrowed_at.try_write_for(TIMEOUT).unwrap() = BorrowInfo::new(Some(caller));
            *self.last_mut_borrowed_at.try_write_for(TIMEOUT).unwrap() =
                BorrowInfo::new(Some(caller));
        }
        self.data.try_write_for(TIMEOUT).unwrap_or_else(|| {
            #[cfg(any(debug_assertions, feature = "debug"))]
            {
                panic!(
                    "Shared::borrow_mut: already borrowed at {}, mutabbly borrowed at {}",
                    self.last_borrowed_at.try_read_for(TIMEOUT).unwrap(),
                    self.last_mut_borrowed_at.try_read_for(TIMEOUT).unwrap()
                )
            }
            #[cfg(not(any(debug_assertions, feature = "debug")))]
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

    pub fn try_borrow(&self) -> Option<RwLockReadGuard<'_, T>> {
        self.data.try_read()
    }

    pub fn try_borrow_mut(&self) -> Option<RwLockWriteGuard<'_, T>> {
        self.data.try_write()
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
        self.borrow().clone()
    }
}
