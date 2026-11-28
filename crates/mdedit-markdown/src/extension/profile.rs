//! Private opt-in scanner-stage markers, without default-build clocks/logging.
pub(super) struct ScanProfile {
    #[cfg(feature = "extension-profiling")]
    call: u64,
}

impl ScanProfile {
    pub(super) fn new() -> Self {
        Self {
            #[cfg(feature = "extension-profiling")]
            call: {
                use std::sync::atomic::{AtomicU64, Ordering};
                static NEXT_CALL: AtomicU64 = AtomicU64::new(1);
                NEXT_CALL.fetch_add(1, Ordering::Relaxed)
            },
        }
    }

    #[inline]
    pub(super) fn measure<T>(
        &self,
        _stage: &'static str,
        _scanner: Option<usize>,
        work: impl FnOnce() -> T,
    ) -> T {
        #[cfg(feature = "extension-profiling")]
        let (label, start) = {
            let label =
                _scanner.map_or_else(|| _stage.to_owned(), |index| format!("{_stage}_{index}"));
            eprintln!("extension-begin,{},{}", self.call, label);
            (label, std::time::Instant::now())
        };
        let result = work();
        #[cfg(feature = "extension-profiling")]
        {
            let elapsed = start.elapsed().as_nanos();
            eprintln!("extension-end,{},{},{}", self.call, label, elapsed);
        }
        result
    }
}
