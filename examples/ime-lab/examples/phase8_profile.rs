//! Headless CPU stage baseline. See scripts/profile-phase8.ps1 and the Phase 8 report.
use std::{
    env,
    error::Error,
    fmt::{self, Write as _},
    hint::black_box,
    io, thread,
    time::{Duration, Instant},
};

use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, Wrap};
use mdedit_core::{Change, ChangeSet, Document, TextRange, TextSize, Transaction, TransactionKind};
use mdedit_live::{ProjectedCaretStops, Projection, ProjectionStatus, RevealContext, RevealPolicy};
use mdedit_markdown::{
    BlockCache, DelimiterResolver, ExtensionIssue, ExtensionMatch, ExtensionSet, MarkdownDialect,
    MarkdownParser, PulldownCmarkParser, obsidian_extension_set,
};

#[derive(Clone, Copy)]
enum Fixture {
    Mixed(&'static str, usize),
    SingleLine,
}

impl Fixture {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "mixed-10kb" => Self::Mixed("mixed-10kb", 10_000),
            "mixed-100kb" => Self::Mixed("mixed-100kb", 100_000),
            "mixed-1mb" => Self::Mixed("mixed-1mb", 1_000_000),
            "mixed-10mb" => Self::Mixed("mixed-10mb", 10_000_000),
            "single-line" => Self::SingleLine,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            Self::Mixed(name, _) => name,
            Self::SingleLine => "single-line",
        }
    }

    fn source(self) -> String {
        match self {
            Self::SingleLine => "a한🙂".repeat(333_333) + "a",
            Self::Mixed(_, bytes) => {
                let stanza = "# Profiling heading\r\n\r\nPlain **bold** and *emphasis* with 한글🙂 [[note]].\r\n\r\n- [ ] task\r\n- item\r\n\r\n> [!note] Title\r\n> body **text**\r\n\r\n| A | B |\r\n| - | - |\r\n| 1 | 2 |\r\n\r\n$$\r\nx^2 + y^2\r\n$$\r\n\r\n```rust\r\nlet n = 42;\r\n```\r\n\r\n---\r\n\r\n";
                let mut source = stanza.repeat(bytes / stanza.len());
                source.extend(std::iter::repeat_n(' ', bytes - source.len()));
                source
            }
        }
    }
}

struct Options {
    fixture: Fixture,
    samples: usize,
    warmups: usize,
    layout: bool,
    policy: RevealPolicy,
    settle_ms: u64,
}
impl Options {
    fn parse() -> Result<Self, Box<dyn Error>> {
        let mut args = env::args().skip(1);
        let name = args.next().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "usage: phase8_profile <mixed-10kb|mixed-100kb|mixed-1mb|mixed-10mb|single-line> [--samples N] [--warmups N] [--no-layout] [--source-visible] [--settle-ms N]"))?;
        let fixture = Fixture::from_name(&name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "unknown fixture"))?;
        let mut result = Self {
            fixture,
            samples: 5,
            warmups: 1,
            layout: true,
            policy: RevealPolicy::ConcealInactive,
            settle_ms: 0,
        };
        while let Some(arg) = args.next() {
            if arg == "--no-layout" {
                result.layout = false;
                continue;
            }
            if arg == "--source-visible" {
                result.policy = RevealPolicy::SourceVisible;
                continue;
            }
            let value = args.next().ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "missing option value")
            })?;
            match arg.as_str() {
                "--samples" => result.samples = value.parse()?,
                "--warmups" => result.warmups = value.parse()?,
                "--settle-ms" => result.settle_ms = value.parse()?,
                _ => {
                    return Err(
                        io::Error::new(io::ErrorKind::InvalidInput, "unknown option").into(),
                    );
                }
            }
        }
        if !(1..=50).contains(&result.samples) || result.warmups > 10 || result.settle_ms > 1000 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "samples 1..50, warmups 0..10, settle-ms 0..1000 required",
            )
            .into());
        }
        Ok(result)
    }
}

struct Clock {
    fixture: &'static str,
    kind: &'static str,
    sample: usize,
}

// Diagnostic FNV-1a over ordered Debug fields, streamed without a large String.
// Pinned to this harness/toolchain; full snapshot tests establish equivalence.
struct ScanDigest(u64);
impl fmt::Write for ScanDigest {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        Ok(())
    }
}

fn scan_digest(matches: &[ExtensionMatch], issues: &[ExtensionIssue]) -> u64 {
    let mut digest = ScanDigest(0xcbf29ce484222325);
    write!(&mut digest, "{matches:?}|{issues:?}").expect("infallible streaming digest");
    digest.0
}
impl Clock {
    fn measure<T>(&self, phase: &str, stage: &str, work: impl FnOnce() -> T) -> T {
        eprintln!(
            "begin,{},{},{},{},{}",
            self.fixture, self.kind, self.sample, phase, stage
        );
        let start = Instant::now();
        let result = black_box(work());
        let elapsed = start.elapsed().as_nanos();
        println!(
            "{},{},{},{},{},{},0",
            self.fixture, self.kind, self.sample, phase, stage, elapsed
        );
        result
    }

    fn metric(&self, phase: &str, name: &str, value: usize) {
        println!(
            "{},meta-{},{},{},{},0,{}",
            self.fixture, self.kind, self.sample, phase, name, value
        );
    }
}

struct Pipeline<'a> {
    extensions: &'a ExtensionSet,
    blocks: BlockCache,
    fonts: Option<&'a mut FontSystem>,
    buffer: Option<Buffer>,
    policy: RevealPolicy,
}

impl<'a> Pipeline<'a> {
    fn new(
        extensions: &'a ExtensionSet,
        mut fonts: Option<&'a mut FontSystem>,
        clock: &Clock,
        policy: RevealPolicy,
    ) -> Self {
        let buffer = fonts.as_deref_mut().map(|font_system| {
            clock.measure("load", "buffer_create", || {
                let mut buffer = Buffer::new(font_system, Metrics::new(22.0, 31.0));
                buffer.set_wrap(Wrap::Word);
                buffer.set_size(Some(1000.0), Some(700.0));
                buffer
            })
        });
        Self {
            extensions,
            blocks: BlockCache::new(),
            fonts,
            buffer,
            policy,
        }
    }

    fn refresh(
        &mut self,
        document: &Document,
        clock: &Clock,
        phase: &str,
    ) -> Result<Projection, Box<dyn Error>> {
        let snapshot = clock.measure(phase, "snapshot", || document.snapshot());
        let syntax = clock.measure(phase, "parse", || {
            PulldownCmarkParser.parse(&snapshot, &MarkdownDialect::obsidian())
        });
        let extensions =
            clock.measure(phase, "scan", || self.extensions.scan(&snapshot, &syntax))?;
        clock.metric(phase, "issues_count", extensions.issues().len());
        let digest = scan_digest(extensions.matches(), extensions.issues());
        clock.metric(phase, "scan_digest_hi", (digest >> 32) as usize);
        clock.metric(
            phase,
            "scan_digest_lo",
            (digest & u64::from(u32::MAX)) as usize,
        );
        let blocks = clock.measure(phase, "blocks", || {
            self.blocks.reconcile(&snapshot, &syntax)
        })?;
        let delimiters = clock.measure(phase, "delimiters", || {
            DelimiterResolver.resolve(&snapshot, &syntax)
        })?;
        let context = RevealContext::new().with_caret(TextSize::new(u32::from(phase == "edit")));
        let projection = clock.measure(phase, "projection", || {
            Projection::build_with_current_extensions(
                &snapshot,
                &syntax,
                &blocks,
                &delimiters,
                &extensions,
                self.extensions,
                self.policy,
                &context,
            )
        })?;
        clock.metric(phase, "blocks_count", blocks.blocks().len());
        clock.metric(phase, "matches_count", extensions.matches().len());
        clock.metric(phase, "delimiters_count", delimiters.spans().len());
        clock.metric(phase, "widgets_count", projection.widgets().len());
        clock.metric(phase, "projected_bytes", projection.text().len());
        clock.metric(
            phase,
            "concealed_ranges",
            projection.map().concealed_ranges().len(),
        );
        clock.metric(
            phase,
            "visible_spans",
            projection.map().visible_spans().len(),
        );
        clock.metric(
            phase,
            "raw_fallback",
            usize::from(matches!(
                projection.status(),
                ProjectionStatus::RawFallback(_)
            )),
        );
        if let (Some(buffer), Some(fonts)) = (&mut self.buffer, self.fonts.as_deref_mut()) {
            let attrs = Attrs::new().family(Family::SansSerif);
            clock.measure(phase, "buffer_set_text", || {
                buffer.set_text(projection.text(), &attrs, Shaping::Advanced, None)
            });
            clock.measure(phase, "shape_viewport", || {
                buffer.shape_until_scroll(fonts, false)
            });
            clock.metric(phase, "visible_layout_runs", buffer.layout_runs().count());
        }
        Ok(projection)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::parse()?;
    let source = options.fixture.source();
    println!("fixture,kind,sample,phase,stage,ns,items");
    let setup = Clock {
        fixture: options.fixture.name(),
        kind: "setup",
        sample: 0,
    };
    setup.metric("setup", "schema_version", 2);
    setup.metric("setup", "source_bytes", source.len());
    setup.metric("setup", "source_chars", source.chars().count());
    let mut fonts = if options.layout {
        Some(setup.measure("setup", "font_system", FontSystem::new))
    } else {
        None
    };
    let extensions = obsidian_extension_set();
    for index in 0..options.warmups + options.samples {
        let warmup = index < options.warmups;
        let clock = Clock {
            fixture: options.fixture.name(),
            kind: if warmup { "warmup" } else { "sample" },
            sample: if warmup {
                index
            } else {
                index - options.warmups
            },
        };
        let mut document = clock.measure("load", "document", || Document::new(&source))?;
        let mut pipeline = Pipeline::new(&extensions, fonts.as_mut(), &clock, options.policy);
        let initial = pipeline.refresh(&document, &clock, "load")?;
        black_box(&initial);
        drop(initial);
        let point = TextSize::new(0);
        let transaction = Transaction::new(
            document.revision(),
            ChangeSet::single(Change::new(TextRange::new(point, point)?, "x")),
            TransactionKind::Typing,
        );
        clock.measure("edit", "transaction", || document.apply(transaction))?;
        let updated = pipeline.refresh(&document, &clock, "edit")?;
        let stops = clock.measure("edit", "caret_stops", || {
            ProjectedCaretStops::from_projection(&updated)
        });
        clock.metric("edit", "caret_count", stops.stops().len());
    }
    eprintln!("complete,{}", options.fixture.name());
    // Outside all timed stages: allow the runner to sample the OS peak counter.
    thread::sleep(Duration::from_millis(options.settle_ms));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Fixture;

    #[test]
    fn mixed_fixtures_reach_exact_byte_sizes_without_truncating_unicode() {
        for (name, bytes) in [
            ("mixed-10kb", 10_000),
            ("mixed-100kb", 100_000),
            ("mixed-1mb", 1_000_000),
            ("mixed-10mb", 10_000_000),
        ] {
            let fixture = Fixture::from_name(name).unwrap();
            let source = fixture.source();
            assert_eq!(source.len(), bytes);
            assert!(source.chars().count() < bytes);
        }
    }

    #[test]
    fn single_line_fixture_has_one_million_unicode_scalars_without_line_endings() {
        let source = Fixture::from_name("single-line").unwrap().source();
        assert_eq!(source.chars().count(), 1_000_000);
        assert_eq!(source.len(), 2_666_665);
        assert!(!source.contains('\n'));
        assert!(!source.contains('\r'));
    }
}
