//! Interior-mutability cell for single-thread (audio-thread) access.
//!
//! VST3's COM-object contract guarantees that the host and plugin only
//! touch a given parameter / event object during
//! `IAudioProcessor::process`, which runs single-threaded on the audio
//! thread. That lets us drop the `Mutex<T>` wrappers traditional
//! implementations use and back those objects with a bare
//! [`UnsafeCell`][core::cell::UnsafeCell] — no lock, no allocation,
//! no contention.
//!
//! [`AudioThreadCell<T>`] is that wrapper. In release builds it compiles
//! to the bare `UnsafeCell`; in debug builds it asserts that every
//! access comes from the same thread that first touched the cell, so a
//! stray call from the UI thread panics immediately instead of silently
//! racing.
//!
//! # Safety contract
//!
//! Every public method requires that the caller be on a single thread
//! for the lifetime of the cell. `Send` moves across threads are fine
//! (the whole [`Vst3Instance`](crate::Vst3Instance) is `Send`); what
//! must not happen is two threads dereferencing the same cell
//! concurrently. See the module docs of
//! [`com::param_queue`](crate::com::param_queue),
//! [`com::param_changes`](crate::com::param_changes), and
//! [`com::event_list`](crate::com::event_list) for the precise
//! discipline each COM object relies on.

use std::cell::UnsafeCell;
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU64, Ordering};

pub struct AudioThreadCell<T> {
    inner: UnsafeCell<T>,
    /// Thread that first borrowed the cell. 0 = unset. Debug-only.
    #[cfg(debug_assertions)]
    owner: AtomicU64,
}

impl<T> AudioThreadCell<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: UnsafeCell::new(val),
            #[cfg(debug_assertions)]
            owner: AtomicU64::new(0),
        }
    }

    /// Clears the owner so the next access can come from a new thread.
    /// Used when the audio stream is torn down and a fresh thread will
    /// drive the next `process` loop.
    pub fn reset_owner(&self) {
        #[cfg(debug_assertions)]
        self.owner.store(0, Ordering::Relaxed);
    }

    /// Returns a mutable reference to the contained value.
    ///
    /// # Panics (debug only)
    /// Panics if called from a different thread than the first borrow.
    #[inline]
    #[allow(clippy::mut_from_ref)]
    #[track_caller]
    pub fn borrow_mut(&self) -> &mut T {
        #[cfg(debug_assertions)]
        self.assert_owner(std::panic::Location::caller());
        // SAFETY: caller upholds single-thread access. Debug builds
        // enforce it via the owner check above.
        unsafe { &mut *self.inner.get() }
    }

    /// Returns a shared reference to the contained value.
    ///
    /// # Panics (debug only)
    /// Panics if called from a different thread than the first borrow.
    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> &T {
        #[cfg(debug_assertions)]
        self.assert_owner(std::panic::Location::caller());
        // SAFETY: same invariant as borrow_mut.
        unsafe { &*self.inner.get() }
    }

    #[cfg(debug_assertions)]
    fn assert_owner(&self, caller: &std::panic::Location<'_>) {
        let current = thread_id();
        match self
            .owner
            .compare_exchange(0, current, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => {}
            Err(existing) if existing == current => {}
            Err(existing) => panic!(
                "AudioThreadCell accessed from thread {current} but was first used from \
                 thread {existing}. This cell must be used from a single thread.\n\
                 Call site: {caller}"
            ),
        }
    }
}

/// Stable numeric ID for the current thread (debug-only diagnostic).
#[cfg(debug_assertions)]
fn thread_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    std::thread_local! {
        static ID: u64 = COUNTER.fetch_add(1, Ordering::Relaxed);
    }
    ID.with(|id| *id)
}

// SAFETY: moving across threads is fine — it's concurrent access from
// two threads that must not happen, which the debug check enforces.
unsafe impl<T: Send> Send for AudioThreadCell<T> {}
unsafe impl<T: Send> Sync for AudioThreadCell<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn borrow_mut_from_same_thread() {
        let cell = AudioThreadCell::new(42u32);
        *cell.borrow_mut() = 100;
        assert_eq!(*cell.borrow(), 100);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn borrow_from_another_thread_panics() {
        use std::sync::Arc;
        let cell = Arc::new(AudioThreadCell::new(0u32));
        let _ = cell.borrow(); // prime owner on this thread

        let cell2 = Arc::clone(&cell);
        let result = std::thread::spawn(move || {
            let _ = cell2.borrow();
        })
        .join();

        assert!(result.is_err(), "expected panic from wrong-thread access");
    }
}
