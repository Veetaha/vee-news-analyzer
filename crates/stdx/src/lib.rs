//! Various missing batteries for Rust

/// Struct that runs the specified closure in its [`Drop`](Drop) impl
struct Guard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Guard<F> {
    fn drop(&mut self) {
        (self.0.take().unwrap())()
    }
}

/// Returns a struct which runs the specified closure in its [`Drop`](Drop) impl
pub fn on_drop<F: FnOnce()>(f: F) -> impl Drop {
    Guard(Some(f))
}

/// Returns a struct which prints execution time info in its [`Drop`](Drop) impl.
///  It logs the inital call as well.
pub fn debug_time_it(label: &'static str) -> impl Drop {
    let start = std::time::Instant::now();
    log::debug!("{}: started", label);
    on_drop(move || log::debug!("{}: {:?}", label, start.elapsed()))
}
