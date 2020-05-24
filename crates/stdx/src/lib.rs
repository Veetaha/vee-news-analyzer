//! Various missing batteries for Rust

use std::ops::Deref;

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

/// Non-empty string that contains at least 1 character other than whitespace
#[derive(Debug)]
pub struct NonHollowString(String);

impl Deref for NonHollowString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for NonHollowString {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 0 {
            return Err("passed string is empty");
        }
        if s.chars().all(char::is_whitespace) {
            return Err("passed string contains only whitespace");
        }
        Ok(Self(s.to_owned()))
    }
}
