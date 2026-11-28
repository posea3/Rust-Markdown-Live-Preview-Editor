//! Buffered parse diagnostic markers; this module is absent from default builds.
use crate::SyntaxSnapshot;
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

pub(super) struct ParseProfile {
    call: u64,
}
impl ParseProfile {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            call: NEXT.fetch_add(1, Ordering::Relaxed),
        }
    }
    pub(super) fn measure<T>(&self, stage: &'static str, work: impl FnOnce() -> T) -> T {
        eprintln!("parse-begin,{},{}", self.call, stage);
        let start = Instant::now();
        let value = work();
        let elapsed = start.elapsed().as_nanos();
        eprintln!("parse-end,{},{},{}", self.call, stage, elapsed);
        value
    }
    pub(super) fn stats(&self, syntax: &SyntaxSnapshot, events: usize) {
        let mut pending = vec![syntax.root()];
        let (mut nodes, mut slots, mut singles) = (0, 0, 0);
        while let Some(node) = pending.pop() {
            nodes += 1;
            slots += node.child_capacity();
            singles += usize::from(node.children().len() == 1);
            pending.extend(node.children());
        }
        eprintln!(
            "parse-stats,{},{},{},{},{}",
            self.call, events, nodes, slots, singles
        );
    }
}
