use std::fmt;
use std::hash::{Hash, Hasher};
use std::thread::ThreadId;
// use std::rc::Rc;
pub use parking_lot::{
    MappedRwLockReadGuard, MappedRwLockWriteGuard, RwLock, RwLockReadGuard, RwLockWriteGuard,
};
use std::cell::RefCell;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use thread_local::ThreadLocal;

const GET_TIMEOUT: Duration = Duration::from_secs(4);
const SET_TIMEOUT: Duration = Duration::from_secs(8);

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
    #[cfg(any(feature = "backtrace", feature = "debug"))]
    last_borrowed_at: Arc<RwLock<BorrowInfo>>,
    #[cfg(any(feature = "backtrace", feature = "debug"))]
    last_mut_borrowed_at: Arc<RwLock<BorrowInfo>>,
    lock_thread_id: Arc<RwLock<Vec<ThreadId>>>,
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
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_borrowed_at: self.last_borrowed_at.clone(),
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_mut_borrowed_at: self.last_mut_borrowed_at.clone(),
            lock_thread_id: self.lock_thread_id.clone(),
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
        self.borrow().fmt(f)
    }
}

impl<T> Shared<T> {
    pub fn new(t: T) -> Self {
        Self {
            data: Arc::new(RwLock::new(t)),
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_borrowed_at: Arc::new(RwLock::new(BorrowInfo::new(None))),
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_mut_borrowed_at: Arc::new(RwLock::new(BorrowInfo::new(None))),
            lock_thread_id: Arc::new(RwLock::new(vec![])),
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
    #[track_caller]
    fn wait_until_unlocked(&self) {
        let mut timeout = GET_TIMEOUT;
        loop {
            let lock_thread = self.lock_thread_id.try_read_for(GET_TIMEOUT).unwrap();
            if lock_thread.is_empty() || lock_thread.last() == Some(&std::thread::current().id()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
            timeout -= Duration::from_millis(1);
            if timeout == Duration::from_secs(0) {
                panic!("timeout");
            }
        }
    }

    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> RwLockReadGuard<'_, T> {
        self.wait_until_unlocked();
        let res = self.data.try_read_for(GET_TIMEOUT).unwrap_or_else(|| {
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            {
                panic!(
                    "Shared::borrow: already borrowed at {}, mutably borrowed at {:?}",
                    self.last_borrowed_at.try_read_for(GET_TIMEOUT).unwrap(),
                    self.last_mut_borrowed_at.try_read_for(GET_TIMEOUT).unwrap()
                )
            }
            #[cfg(not(any(feature = "backtrace", feature = "debug")))]
            {
                panic!("Shared::borrow: already borrowed")
            }
        });
        #[cfg(any(feature = "backtrace", feature = "debug"))]
        {
            *self.last_borrowed_at.try_write_for(GET_TIMEOUT).unwrap() =
                BorrowInfo::new(Some(std::panic::Location::caller()));
        }
        res
    }

    #[inline]
    #[track_caller]
    pub fn borrow_mut(&self) -> RwLockWriteGuard<'_, T> {
        self.wait_until_unlocked();
        let res = self.data.try_write_for(SET_TIMEOUT).unwrap_or_else(|| {
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            {
                panic!(
                    "Shared::borrow_mut: already borrowed at {}, mutabbly borrowed at {}",
                    self.last_borrowed_at.try_read_for(SET_TIMEOUT).unwrap(),
                    self.last_mut_borrowed_at.try_read_for(SET_TIMEOUT).unwrap()
                )
            }
            #[cfg(not(any(feature = "backtrace", feature = "debug")))]
            {
                panic!("Shared::borrow_mut: already borrowed")
            }
        });
        #[cfg(any(feature = "backtrace", feature = "debug"))]
        {
            let caller = std::panic::Location::caller();
            *self.last_borrowed_at.try_write_for(SET_TIMEOUT).unwrap() =
                BorrowInfo::new(Some(caller));
            *self
                .last_mut_borrowed_at
                .try_write_for(SET_TIMEOUT)
                .unwrap() = BorrowInfo::new(Some(caller));
        }
        res
    }

    /// Lock the data and deny access from other threads.
    /// Locking can be done any number of times and will not be available until unlocked the same number of times.
    pub fn inter_thread_lock(&self) {
        let mut lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        loop {
            if lock_thread.is_empty() || lock_thread.last() == Some(&std::thread::current().id()) {
                break;
            }
            drop(lock_thread);
            lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        }
        lock_thread.push(std::thread::current().id());
    }

    #[track_caller]
    pub fn inter_thread_unlock(&self) {
        let mut lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        loop {
            if lock_thread.is_empty() {
                panic!("not locked");
            } else if lock_thread.last() == Some(&std::thread::current().id()) {
                break;
            }
            drop(lock_thread);
            lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        }
        lock_thread.pop();
    }

    pub fn inter_thread_unlock_using_id(&self, id: ThreadId) {
        let mut lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        loop {
            if lock_thread.is_empty() {
                panic!("not locked");
            } else if lock_thread.last() == Some(&id)
                || lock_thread.last() == Some(&std::thread::current().id())
            {
                break;
            }
            drop(lock_thread);
            lock_thread = self.lock_thread_id.try_write_for(GET_TIMEOUT).unwrap();
        }
        lock_thread.pop();
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

    pub fn try_borrow_for(&self, timeout: Duration) -> Option<RwLockReadGuard<'_, T>> {
        self.data.try_read_for(timeout)
    }

    pub fn try_borrow_mut_for(&self, timeout: Duration) -> Option<RwLockWriteGuard<'_, T>> {
        self.data.try_write_for(timeout)
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

/// Thread-local objects that can be shared among threads.
/// The initial value can be shared globally, but the changes are not reflected in other threads.
/// If you want to reflect the changes in other threads, you need to call `update_init`.
/// Otherwise, this behaves as a `RefCell`.
#[derive(Clone)]
pub struct Forkable<T: Send + Clone> {
    data: Arc<ThreadLocal<RefCell<T>>>,
    init: Arc<T>,
    #[cfg(any(feature = "backtrace", feature = "debug"))]
    last_borrowed_at: Arc<ThreadLocal<RefCell<BorrowInfo>>>,
    #[cfg(any(feature = "backtrace", feature = "debug"))]
    last_mut_borrowed_at: Arc<ThreadLocal<RefCell<BorrowInfo>>>,
}

impl<T: fmt::Debug + Send + Clone> fmt::Debug for Forkable<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: fmt::Display + Send + Clone> fmt::Display for Forkable<T>
where
    RefCell<T>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Send + Clone> Deref for Forkable<T> {
    type Target = RefCell<T>;
    fn deref(&self) -> &Self::Target {
        self.data
            .get_or(|| RefCell::new(self.init.clone().as_ref().clone()))
    }
}

impl<T: Send + Clone> Forkable<T> {
    pub fn new(init: T) -> Self {
        Self {
            data: Arc::new(ThreadLocal::new()),
            init: Arc::new(init),
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_borrowed_at: Arc::new(ThreadLocal::new()),
            #[cfg(any(feature = "backtrace", feature = "debug"))]
            last_mut_borrowed_at: Arc::new(ThreadLocal::new()),
        }
    }

    pub fn update_init(&mut self) {
        let clone = self.clone_inner();
        // NG: self.init = Arc::new(clone);
        *self = Self::new(clone);
    }

    pub fn clone_inner(&self) -> T {
        self.deref().borrow().clone()
    }

    #[track_caller]
    pub fn borrow(&self) -> std::cell::Ref<'_, T> {
        match self.deref().try_borrow() {
            Ok(res) => {
                #[cfg(any(feature = "backtrace", feature = "debug"))]
                {
                    *self
                        .last_borrowed_at
                        .get_or(|| RefCell::new(BorrowInfo::new(None)))
                        .borrow_mut() = BorrowInfo::new(Some(std::panic::Location::caller()));
                }
                res
            }
            Err(err) => {
                #[cfg(any(feature = "backtrace", feature = "debug"))]
                {
                    panic!(
                        "Forkable::borrow: already borrowed at {}, mutably borrowed at {} ({err})",
                        self.last_borrowed_at
                            .get_or(|| RefCell::new(BorrowInfo::new(None)))
                            .borrow(),
                        self.last_mut_borrowed_at
                            .get_or(|| RefCell::new(BorrowInfo::new(None)))
                            .borrow()
                    )
                }
                #[cfg(not(any(feature = "backtrace", feature = "debug")))]
                {
                    panic!("Forkable::borrow: {err:?}")
                }
            }
        }
    }

    #[track_caller]
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, T> {
        match self.deref().try_borrow_mut() {
            Ok(res) => {
                #[cfg(any(feature = "backtrace", feature = "debug"))]
                {
                    let caller = std::panic::Location::caller();
                    *self
                        .last_borrowed_at
                        .get_or(|| RefCell::new(BorrowInfo::new(None)))
                        .borrow_mut() = BorrowInfo::new(Some(caller));
                    *self
                        .last_mut_borrowed_at
                        .get_or(|| RefCell::new(BorrowInfo::new(None)))
                        .borrow_mut() = BorrowInfo::new(Some(caller));
                }
                res
            }
            Err(err) => {
                #[cfg(any(feature = "backtrace", feature = "debug"))]
                {
                    panic!(
                        "Forkable::borrow_mut: already borrowed at {}, mutably borrowed at {} ({err})",
                        self.last_borrowed_at
                            .get_or(|| RefCell::new(BorrowInfo::new(None)))
                            .borrow(),
                        self.last_mut_borrowed_at
                            .get_or(|| RefCell::new(BorrowInfo::new(None)))
                            .borrow()
                    )
                }
                #[cfg(not(any(feature = "backtrace", feature = "debug")))]
                {
                    panic!("Forkable::borrow_mut: {err:?}")
                }
            }
        }
    }
}
