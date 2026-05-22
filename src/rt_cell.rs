//! Interior-mutability cell for audio-thread-only access.
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
//! to the bare `UnsafeCell`; in debug builds it asserts that no second
//! borrow is live on a *different* thread, so a stray call from the UI
//! thread panics immediately instead of silently racing. Sequential
//! access from different threads (e.g. OS audio stacks that migrate the
//! callback between invocations, notably CoreAudio on macOS) is allowed
//! — the contract is "one borrow at a time", not "always the same
//! thread".
//!
//! # Safety contract
//!
//! Every public method requires that at any instant at most one borrow
//! of the cell is live. `Send` moves across threads are fine (the whole
//! [`Vst3Instance`](crate::Vst3Instance) is `Send`); what must not
//! happen is two threads dereferencing the same cell concurrently. See
//! the module docs of [`com::param_queue`](crate::com::param_queue),
//! [`com::param_changes`](crate::com::param_changes), and
//! [`com::event_list`](crate::com::event_list) for the precise
//! discipline each COM object relies on.

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicBool, Ordering};

pub struct AudioThreadCell<T> {
    inner: UnsafeCell<T>,
    /// Set while a borrow guard is live. Debug-only.
    #[cfg(debug_assertions)]
    in_use: AtomicBool,
}

impl<T> AudioThreadCell<T> {
    pub const fn new(val: T) -> Self {
        Self {
            inner: UnsafeCell::new(val),
            #[cfg(debug_assertions)]
            in_use: AtomicBool::new(false),
        }
    }

    /// No-op kept for source compatibility — the cell no longer pins an
    /// owner thread, so device switching needs no reset. Will be removed
    /// once all callers have dropped their `reset_owner()` calls.
    #[inline]
    pub fn reset_owner(&self) {}

    /// Borrow the cell mutably for the lifetime of the returned guard.
    ///
    /// # Panics (debug only)
    /// Panics if another borrow is already live on a different thread.
    #[inline]
    #[track_caller]
    pub fn borrow_mut(&self) -> BorrowGuard<'_, T> {
        #[cfg(debug_assertions)]
        self.acquire(std::panic::Location::caller());
        BorrowGuard { cell: self }
    }

    /// Borrow the cell shared for the lifetime of the returned guard.
    ///
    /// Note: the contract still allows only one borrow at a time, so
    /// this is just a convenience for `&T` access — it does not enable
    /// multiple concurrent readers.
    ///
    /// # Panics (debug only)
    /// Panics if another borrow is already live on a different thread.
    #[inline]
    #[track_caller]
    pub fn borrow(&self) -> BorrowRef<'_, T> {
        #[cfg(debug_assertions)]
        self.acquire(std::panic::Location::caller());
        BorrowRef { cell: self }
    }

    #[cfg(debug_assertions)]
    #[inline]
    fn acquire(&self, caller: &std::panic::Location<'_>) {
        if self.in_use.swap(true, Ordering::Acquire) {
            panic!(
                "AudioThreadCell concurrent borrow detected — another borrow is \
                 already live. The contract is one borrow at a time.\n\
                 Call site: {caller}"
            );
        }
    }

    #[cfg(debug_assertions)]
    #[inline]
    fn release(&self) {
        self.in_use.store(false, Ordering::Release);
    }
}

/// Mutable borrow guard returned by [`AudioThreadCell::borrow_mut`].
pub struct BorrowGuard<'a, T> {
    cell: &'a AudioThreadCell<T>,
}

impl<T> Deref for BorrowGuard<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: caller upholds the single-borrow invariant; debug
        // builds additionally enforce it via the `in_use` flag set in
        // `acquire`.
        unsafe { &*self.cell.inner.get() }
    }
}

impl<T> DerefMut for BorrowGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as above; `&mut self` on the guard plus the in-use
        // flag ensure no other borrow can observe this reference.
        unsafe { &mut *self.cell.inner.get() }
    }
}

impl<T> Drop for BorrowGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        self.cell.release();
    }
}

/// Shared borrow guard returned by [`AudioThreadCell::borrow`].
pub struct BorrowRef<'a, T> {
    cell: &'a AudioThreadCell<T>,
}

impl<T> Deref for BorrowRef<'_, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &T {
        // SAFETY: see `BorrowGuard::deref`.
        unsafe { &*self.cell.inner.get() }
    }
}

impl<T> Drop for BorrowRef<'_, T> {
    #[inline]
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        self.cell.release();
    }
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
    fn sequential_borrows_across_threads_are_ok() {
        // After one thread's borrow drops, another thread may borrow —
        // that is the macOS CoreAudio scenario the old check tripped on.
        use std::sync::Arc;
        let cell = Arc::new(AudioThreadCell::new(0u32));
        *cell.borrow_mut() = 7;

        let cell2 = Arc::clone(&cell);
        std::thread::spawn(move || {
            *cell2.borrow_mut() = 9;
        })
        .join()
        .unwrap();

        assert_eq!(*cell.borrow(), 9);
    }

    #[test]
    #[cfg(debug_assertions)]
    fn concurrent_borrows_panic() {
        use std::sync::{Arc, Barrier};
        let cell = Arc::new(AudioThreadCell::new(0u32));
        let barrier = Arc::new(Barrier::new(2));

        let cell2 = Arc::clone(&cell);
        let barrier2 = Arc::clone(&barrier);
        let other = std::thread::spawn(move || {
            let _g = cell2.borrow_mut();
            barrier2.wait();
            // Hold the guard while the main thread tries to borrow.
            std::thread::sleep(std::time::Duration::from_millis(50));
        });

        barrier.wait();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _g = cell.borrow_mut();
        }));

        other.join().unwrap();
        assert!(
            result.is_err(),
            "expected panic from concurrent borrow while another borrow was live"
        );
    }
}
