// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Torgeir Børresen <tb@starkad.no>
// Rust port of Google's S2 Geometry library — a derivative work, modified from
// the upstream Apache-2.0 source(s) below (Copyright Google Inc.). See LICENSE.
//   - C++:  google/s2geometry

#![expect(
    clippy::cast_possible_wrap,
    reason = "usize -> i64 for byte counts — always fits on 64-bit"
)]
// S2MemoryTracker: tracks and limits memory usage of S2 operations.
//
// Provides:
//  - Current and maximum tracked memory usage
//  - Cancellation if a memory limit is exceeded
//  - Optional periodic callback for external cancellation checks
//
// C++: `s2memory_tracker.h`
//
// # Example
//
// ```
// use std::sync::{Arc, Mutex};
// use s2rst::s2::memory_tracker::{S2MemoryTracker, Client};
//
// let tracker = Arc::new(Mutex::new(S2MemoryTracker::new()));
// tracker.lock().unwrap().set_limit(500 << 20); // 500 MB
//
// // Multiple clients can share the same tracker (like C++).
// let mut client_a = Client::new(Arc::clone(&tracker));
// let mut client_b = Client::new(Arc::clone(&tracker));
// client_a.tally(100);
// client_b.tally(200);
// assert_eq!(tracker.lock().unwrap().usage_bytes(), 300);
// ```

use std::sync::{Arc, Mutex};

use super::builder::{S2Error, S2ErrorCode};

/// Tracks and limits memory usage of S2 operations.
///
/// Wrap in `Arc<Mutex<S2MemoryTracker>>` to share across multiple
/// [`Client`] objects, matching the C++ pattern where several
/// `S2MemoryTracker::Client*` point to the same tracker.
///
/// C++: `S2MemoryTracker`
pub struct S2MemoryTracker {
    usage_bytes: i64,
    max_usage_bytes: i64,
    limit_bytes: i64,
    alloc_bytes: i64,
    error: S2Error,
    callback: Option<Box<dyn FnMut() + Send>>,
    callback_alloc_delta_bytes: i64,
    callback_alloc_limit_bytes: i64,
}

impl std::fmt::Debug for S2MemoryTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("S2MemoryTracker")
            .field("usage_bytes", &self.usage_bytes)
            .field("max_usage_bytes", &self.max_usage_bytes)
            .field("limit_bytes", &self.limit_bytes)
            .finish_non_exhaustive()
    }
}

/// Indicates unlimited memory usage.
pub const NO_LIMIT: i64 = i64::MAX;

impl S2MemoryTracker {
    /// Creates a new tracker with no memory limit.
    pub fn new() -> Self {
        S2MemoryTracker {
            usage_bytes: 0,
            max_usage_bytes: 0,
            limit_bytes: NO_LIMIT,
            alloc_bytes: 0,
            error: S2Error::ok(),
            callback: None,
            callback_alloc_delta_bytes: 0,
            callback_alloc_limit_bytes: NO_LIMIT,
        }
    }

    /// Returns the current tracked memory usage in bytes.
    ///
    /// **Caveat**: When an operation is cancelled, this value may be inaccurate
    /// because it reports attempted rather than actual allocation.
    pub fn usage_bytes(&self) -> i64 {
        self.usage_bytes
    }

    /// Returns the maximum tracked memory usage in bytes.
    pub fn max_usage_bytes(&self) -> i64 {
        self.max_usage_bytes
    }

    /// Returns the current memory limit in bytes.
    pub fn limit_bytes(&self) -> i64 {
        self.limit_bytes
    }

    /// Sets the memory limit in bytes. When tracked usage exceeds this value,
    /// a `ResourceExhausted` error is generated and the operation is cancelled.
    /// Use [`NO_LIMIT`] for unlimited.
    pub fn set_limit(&mut self, limit_bytes: i64) {
        self.limit_bytes = limit_bytes;
    }

    /// Returns the current error status.
    pub fn error(&self) -> &S2Error {
        &self.error
    }

    /// Returns true if no memory tracking errors have occurred.
    pub fn ok(&self) -> bool {
        self.error.is_ok()
    }

    /// Sets the error status, requesting cancellation of the current operation.
    pub fn set_error(&mut self, error: S2Error) {
        self.error = error;
    }

    /// Returns the callback allocation delta in bytes.
    pub fn callback_alloc_delta_bytes(&self) -> i64 {
        self.callback_alloc_delta_bytes
    }

    /// Sets a periodic callback invoked after every `delta_bytes` of cumulative
    /// allocation. The callback can cancel the operation by calling `set_error`.
    pub fn set_periodic_callback(
        &mut self,
        callback_alloc_delta_bytes: i64,
        callback: impl FnMut() + Send + 'static,
    ) {
        self.callback_alloc_delta_bytes = callback_alloc_delta_bytes;
        self.callback = Some(Box::new(callback));
        self.callback_alloc_limit_bytes = self.alloc_bytes + callback_alloc_delta_bytes;
    }

    /// Resets `usage/max_usage` to zero and clears any error.
    /// Leaves limit and callback settings unchanged.
    pub fn reset(&mut self) {
        self.error = S2Error::ok();
        self.usage_bytes = 0;
        self.max_usage_bytes = 0;
        self.alloc_bytes = 0;
        self.callback_alloc_limit_bytes = self.callback_alloc_delta_bytes;
    }

    /// Records `delta_bytes` of memory use (positive = allocation, negative = free).
    /// Returns false if the current operation should be cancelled.
    pub fn tally(&mut self, delta_bytes: i64) -> bool {
        self.usage_bytes += delta_bytes;
        self.alloc_bytes += delta_bytes.max(0);
        if self.usage_bytes > self.max_usage_bytes {
            self.max_usage_bytes = self.usage_bytes;
        }
        if self.usage_bytes > self.limit_bytes && self.ok() {
            self.set_limit_exceeded_error();
        }
        if self.callback.is_some() && self.alloc_bytes >= self.callback_alloc_limit_bytes {
            self.callback_alloc_limit_bytes = self.alloc_bytes + self.callback_alloc_delta_bytes;
            if self.ok() {
                // Temporarily take the callback to call it, then put it back.
                // This avoids borrow issues since the callback might call set_error.
                let mut cb = self.callback.take();
                if let Some(ref mut f) = cb {
                    f();
                }
                self.callback = cb;
            }
        }
        self.ok()
    }

    fn set_limit_exceeded_error(&mut self) {
        self.error = S2Error::new(
            S2ErrorCode::ResourceExhausted,
            format!(
                "Memory limit exceeded (tracked usage {} bytes, limit {} bytes)",
                self.usage_bytes, self.limit_bytes
            ),
        );
    }
}

impl Default for S2MemoryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// A client of [`S2MemoryTracker`] that tracks its own usage and updates the
/// shared tracker. Multiple clients can point to the same tracker, matching
/// the C++ pattern where `S2BooleanOperation` and `S2Builder` each create
/// their own `Client` against a shared `S2MemoryTracker`.
///
/// When dropped, the client's usage is automatically subtracted from the
/// tracker (under the assumption that the associated operation has been
/// destroyed as well).
///
/// C++: `S2MemoryTracker::Client`
#[derive(Debug)]
pub struct Client {
    tracker: Option<Arc<Mutex<S2MemoryTracker>>>,
    /// The client's own tracked usage (subset of the tracker total).
    client_usage_bytes: i64,
}

/// Locks a tracker, recovering from poisoned state (a previous thread panic).
/// The data is still valid since `S2MemoryTracker` has no complex invariants
/// that could be violated by a partial update.
pub fn lock_tracker(t: &Mutex<S2MemoryTracker>) -> std::sync::MutexGuard<'_, S2MemoryTracker> {
    t.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Client {
    /// Creates a client with no tracker (inactive). Memory is not tracked.
    pub fn inactive() -> Self {
        Client {
            tracker: None,
            client_usage_bytes: 0,
        }
    }

    /// Creates a client tracking memory via the given shared tracker.
    ///
    /// Multiple clients can share the same `Arc<Mutex<S2MemoryTracker>>`,
    /// matching C++ where several `Client*` point to one `S2MemoryTracker`.
    pub fn new(tracker: Arc<Mutex<S2MemoryTracker>>) -> Self {
        Client {
            tracker: Some(tracker),
            client_usage_bytes: 0,
        }
    }

    /// Reinitializes this client to use a (possibly different) tracker.
    ///
    /// Any existing usage is first subtracted from the old tracker, then
    /// the current `client_usage_bytes` is transferred to the new tracker.
    ///
    /// C++: `S2MemoryTracker::Client::Init`
    pub fn init(&mut self, tracker: Arc<Mutex<S2MemoryTracker>>) {
        let usage = self.client_usage_bytes;
        self.tally(-usage);
        self.tracker = Some(tracker);
        self.tally(usage);
    }

    /// Returns a clone of the shared tracker reference, if active.
    pub fn tracker(&self) -> Option<Arc<Mutex<S2MemoryTracker>>> {
        self.tracker.as_ref().map(Arc::clone)
    }

    /// Returns true if this client has an associated tracker.
    pub fn is_active(&self) -> bool {
        self.tracker.is_some()
    }

    /// Returns true if no errors have occurred.
    pub fn ok(&self) -> bool {
        match &self.tracker {
            Some(t) => lock_tracker(t).ok(),
            None => true,
        }
    }

    /// Returns the tracker's current error.
    pub fn error(&self) -> S2Error {
        match &self.tracker {
            Some(t) => lock_tracker(t).error().clone(),
            None => S2Error::ok(),
        }
    }

    /// Returns the current tracked memory usage of the whole tracker.
    pub fn usage_bytes(&self) -> i64 {
        match &self.tracker {
            Some(t) => lock_tracker(t).usage_bytes(),
            None => 0,
        }
    }

    /// Returns the current tracked memory usage of this client only.
    pub fn client_usage_bytes(&self) -> i64 {
        self.client_usage_bytes
    }

    /// Records `delta_bytes` of memory use. Returns false if the operation
    /// should be cancelled.
    pub fn tally(&mut self, delta_bytes: i64) -> bool {
        self.client_usage_bytes += delta_bytes;
        match &self.tracker {
            Some(t) => lock_tracker(t).tally(delta_bytes),
            None => true,
        }
    }

    /// Records temporary memory allocation: adds then immediately subtracts.
    /// Returns false if the operation should be cancelled.
    ///
    /// C++: `S2MemoryTracker::Client::TallyTemp`
    pub fn tally_temp(&mut self, delta_bytes: i64) -> bool {
        self.tally(delta_bytes);
        self.tally(-delta_bytes)
    }

    /// Tracks allocation for `n` additional elements of size `elem_size`.
    /// Returns false if the operation should be cancelled.
    pub fn tally_elements(&mut self, n: usize, elem_size: usize) -> bool {
        self.tally((n * elem_size) as i64)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(ref t) = self.tracker {
            lock_tracker(t).tally(-self.client_usage_bytes);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn test_basic_tracking() {
        let mut tracker = S2MemoryTracker::new();
        tracker.set_limit(1000);

        assert!(tracker.ok());
        assert_eq!(tracker.usage_bytes(), 0);

        tracker.tally(500);
        assert!(tracker.ok());
        assert_eq!(tracker.usage_bytes(), 500);
        assert_eq!(tracker.max_usage_bytes(), 500);

        tracker.tally(-200);
        assert!(tracker.ok());
        assert_eq!(tracker.usage_bytes(), 300);
        assert_eq!(tracker.max_usage_bytes(), 500);
    }

    #[test]
    fn test_limit_exceeded() {
        let mut tracker = S2MemoryTracker::new();
        tracker.set_limit(100);

        assert!(tracker.tally(50));
        assert!(tracker.ok());

        assert!(!tracker.tally(60));
        assert!(!tracker.ok());
        assert_eq!(tracker.error().code, S2ErrorCode::ResourceExhausted);
    }

    #[test]
    fn test_no_limit() {
        let mut tracker = S2MemoryTracker::new();
        // Default is NO_LIMIT
        assert!(tracker.tally(i64::MAX / 2));
        assert!(tracker.ok());
    }

    #[test]
    fn test_periodic_callback() {
        let mut tracker = S2MemoryTracker::new();
        let callback_count = Arc::new(AtomicI32::new(0));
        let count = Arc::clone(&callback_count);

        // Callback every 0 bytes (every tally call).
        tracker.set_periodic_callback(0, move || {
            count.fetch_add(1, Ordering::Relaxed);
        });
        assert_eq!(tracker.callback_alloc_delta_bytes(), 0);

        tracker.tally(0);
        assert_eq!(callback_count.load(Ordering::Relaxed), 1);

        // Negative tallies still trigger callback (based on cumulative alloc).
        tracker.tally(-10);
        assert_eq!(callback_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_periodic_callback_interval() {
        let mut tracker = S2MemoryTracker::new();
        let callback_count = Arc::new(AtomicI32::new(0));
        let count = Arc::clone(&callback_count);

        tracker.set_periodic_callback(100, move || {
            count.fetch_add(1, Ordering::Relaxed);
        });

        tracker.tally(99);
        assert_eq!(callback_count.load(Ordering::Relaxed), 0);

        tracker.tally(1);
        assert_eq!(callback_count.load(Ordering::Relaxed), 1);

        // Free and re-allocate: callback is based on cumulative allocation.
        tracker.tally(-50);
        tracker.tally(50);
        tracker.tally(-50);
        tracker.tally(49);
        assert_eq!(callback_count.load(Ordering::Relaxed), 1);

        tracker.tally(1);
        assert_eq!(callback_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_reset() {
        let mut tracker = S2MemoryTracker::new();
        tracker.set_limit(100);
        tracker.tally(200);
        assert!(!tracker.ok());

        tracker.reset();
        assert!(tracker.ok());
        assert_eq!(tracker.usage_bytes(), 0);
        assert_eq!(tracker.max_usage_bytes(), 0);
        assert_eq!(tracker.limit_bytes(), 100); // Limit preserved.
    }

    #[test]
    fn test_client_basic() {
        let tracker = Arc::new(Mutex::new(S2MemoryTracker::new()));
        tracker.lock().expect("lock").set_limit(1000);
        {
            let mut client = Client::new(Arc::clone(&tracker));
            assert!(client.is_active());
            assert!(client.tally(500));
            assert_eq!(client.client_usage_bytes(), 500);
            assert_eq!(client.usage_bytes(), 500);
        }
        // After client is dropped, usage should be subtracted.
        assert_eq!(tracker.lock().expect("lock").usage_bytes(), 0);
        assert_eq!(tracker.lock().expect("lock").max_usage_bytes(), 500);
    }

    #[test]
    fn test_client_inactive() {
        let mut client = Client::inactive();
        assert!(!client.is_active());
        assert!(client.ok());
        assert!(client.tally(1000));
    }

    #[test]
    fn test_client_tally_temp() {
        let tracker = Arc::new(Mutex::new(S2MemoryTracker::new()));
        tracker.lock().expect("lock").set_limit(1000);
        {
            let mut client = Client::new(Arc::clone(&tracker));
            assert!(client.tally_temp(500));
            assert_eq!(client.client_usage_bytes(), 0);
        }
        // Max should have recorded the peak.
        assert_eq!(tracker.lock().expect("lock").max_usage_bytes(), 500);
    }

    #[test]
    fn test_multi_client_shared_tracker() {
        // C++ pattern: multiple Client* pointing to the same S2MemoryTracker.
        let tracker = Arc::new(Mutex::new(S2MemoryTracker::new()));
        tracker.lock().expect("lock").set_limit(1000);

        let mut client_a = Client::new(Arc::clone(&tracker));
        let mut client_b = Client::new(Arc::clone(&tracker));

        // Both clients contribute to the same tracker total.
        assert!(client_a.tally(300));
        assert!(client_b.tally(400));
        assert_eq!(tracker.lock().expect("lock").usage_bytes(), 700);
        assert_eq!(client_a.client_usage_bytes(), 300);
        assert_eq!(client_b.client_usage_bytes(), 400);

        // Dropping client_a subtracts its usage.
        drop(client_a);
        assert_eq!(tracker.lock().expect("lock").usage_bytes(), 400);

        // client_b can still allocate.
        assert!(client_b.tally(100));
        assert_eq!(tracker.lock().expect("lock").usage_bytes(), 500);
        assert_eq!(client_b.client_usage_bytes(), 500);

        // Dropping client_b subtracts its usage.
        drop(client_b);
        assert_eq!(tracker.lock().expect("lock").usage_bytes(), 0);
        assert_eq!(tracker.lock().expect("lock").max_usage_bytes(), 700);
    }

    #[test]
    fn test_multi_client_limit_exceeded() {
        // When one client exceeds the limit, the other sees the error too.
        let tracker = Arc::new(Mutex::new(S2MemoryTracker::new()));
        tracker.lock().expect("lock").set_limit(500);

        let mut client_a = Client::new(Arc::clone(&tracker));
        let mut client_b = Client::new(Arc::clone(&tracker));

        assert!(client_a.tally(300));
        // client_b pushes over the limit.
        assert!(!client_b.tally(300));
        // Both clients now report not-ok.
        assert!(!client_a.ok());
        assert!(!client_b.ok());
        assert_eq!(
            tracker.lock().expect("lock").error().code,
            S2ErrorCode::ResourceExhausted
        );
    }

    #[test]
    fn test_client_init_transfer() {
        // C++: Client::Init — re-initialize client to a different tracker.
        let tracker1 = Arc::new(Mutex::new(S2MemoryTracker::new()));
        let tracker2 = Arc::new(Mutex::new(S2MemoryTracker::new()));

        let mut client = Client::new(Arc::clone(&tracker1));
        client.tally(100);
        assert_eq!(tracker1.lock().expect("lock").usage_bytes(), 100);

        // Transfer client to tracker2. Old usage subtracted from tracker1,
        // then re-added to tracker2.
        client.init(Arc::clone(&tracker2));
        assert_eq!(tracker1.lock().expect("lock").usage_bytes(), 0);
        assert_eq!(tracker2.lock().expect("lock").usage_bytes(), 100);
        assert_eq!(client.client_usage_bytes(), 100);

        // Further tallying goes to tracker2.
        client.tally(50);
        assert_eq!(tracker2.lock().expect("lock").usage_bytes(), 150);
    }
}
