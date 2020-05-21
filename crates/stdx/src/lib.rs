//! Various missing batteries for Rust

/// Struct that runs the specified closure on its `drop()` call
pub struct Guard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> Drop for Guard<F> {
    fn drop(&mut self) {
        (self.0.take().unwrap())()
    }
}

/// Returns a struct which runs the specified closure on its `drop()` call
pub fn on_drop<F: FnOnce()>(f: F) -> Guard<F> {
    Guard(Some(f))
}

pub fn debug_time_it(label: &'static str) -> impl Drop {
    let start = std::time::Instant::now();
    log::debug!("{}: started", label);
    on_drop(move || log::debug!("{}: {:?}", label, start.elapsed()))
}
