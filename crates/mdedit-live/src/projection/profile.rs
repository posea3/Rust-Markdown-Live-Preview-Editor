//! Private opt-in build-stage markers. Default builds contain no clocks/logging.

pub(super) struct BuildProfile {
    #[cfg(feature = "projection-profiling")]
    call: u64,
}

impl BuildProfile {
    pub(super) fn new() -> Self {
        Self {
            #[cfg(feature = "projection-profiling")]
            call: {
                use std::sync::atomic::{AtomicU64, Ordering};
                static NEXT_CALL: AtomicU64 = AtomicU64::new(1);
                NEXT_CALL.fetch_add(1, Ordering::Relaxed)
            },
        }
    }

    #[inline]
    pub(super) fn measure<T>(&self, _stage: &'static str, work: impl FnOnce() -> T) -> T {
        #[cfg(feature = "projection-profiling")]
        let start = {
            eprintln!("projection-begin,{},{}", self.call, _stage);
            std::time::Instant::now()
        };
        let result = work();
        #[cfg(feature = "projection-profiling")]
        {
            let elapsed = start.elapsed().as_nanos();
            eprintln!("projection-end,{},{},{}", self.call, _stage, elapsed);
        }
        result
    }
}
