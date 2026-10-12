//! A clock that works in the browser build too. `std::time::Instant` panics on wasm32-unknown-unknown, which has no
//! clock of its own, so there the page lends one: the module imports `env.fsc_now_ms` (the page's
//! `performance.now()`). Everywhere else this is the standard `Instant`.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

#[cfg(target_arch = "wasm32")]
mod web {
    use std::time::Duration;

    #[link(wasm_import_module = "env")]
    extern "C" {
        fn fsc_now_ms() -> f64;
    }

    /// Milliseconds from the page's clock, for timing commands; as `std::time::Instant`, only `now` and `elapsed`.
    #[derive(Clone, Copy, Debug)]
    pub struct Instant(f64);

    impl Instant {
        pub fn now() -> Instant {
            // SAFETY: the page provides this import; it takes nothing and returns a number.
            Instant(unsafe { fsc_now_ms() })
        }

        pub fn elapsed(&self) -> Duration {
            Duration::from_secs_f64(((Instant::now().0 - self.0) / 1000.0).max(0.0))
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use web::Instant;
