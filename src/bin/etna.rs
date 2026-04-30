// ETNA workload runner for winnow.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: HexUintOverflowErrors | IteratorNoPanicAfterDone
//             | RepeatNLoopCheckErrors | All
//
// Each run emits a single JSON line on stdout with fields:
//   status, tests, discards, time, counterexample, error, tool, property.

use crabcheck::quickcheck as crabcheck_qc;
use hegel::{generators as hgen, Hegel, Settings as HegelSettings};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestRunner};
use quickcheck::{QuickCheck, ResultStatus, TestResult};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use winnow::etna::{
    property_hex_uint_overflow_errors, property_iterator_no_panic_after_done,
    property_repeat_n_loop_check_errors, PropertyResult,
};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &[
    "HexUintOverflowErrors",
    "IteratorNoPanicAfterDone",
    "RepeatNLoopCheckErrors",
];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    let mut final_status: Result<(), String> = Ok(());
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if r.is_err() && final_status.is_ok() {
            final_status = r;
        }
    }
    (final_status, total)
}

// ---- etna (deterministic witness inputs) ----

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "HexUintOverflowErrors" => to_err(property_hex_uint_overflow_errors(b"123456789".to_vec())),
        "IteratorNoPanicAfterDone" => {
            to_err(property_iterator_no_panic_after_done(b"ab".to_vec()))
        }
        "RepeatNLoopCheckErrors" => to_err(property_repeat_n_loop_check_errors(2)),
        _ => {
            return (
                Err(format!("Unknown property for etna: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    (result, Metrics { inputs: 1, elapsed_us })
}

// ---- proptest ----

fn hex_bytes_strategy() -> BoxedStrategy<Vec<u8>> {
    // Bias toward all-hex inputs of length 1..=16 so the property exercises
    // both the overflow and non-overflow branches.
    proptest::collection::vec(
        prop_oneof![
            (0u8..10).prop_map(|d| b'0' + d),
            (0u8..6).prop_map(|d| b'a' + d),
            (0u8..6).prop_map(|d| b'A' + d),
            any::<u8>(),
        ],
        0..=16,
    )
    .boxed()
}

fn iterator_input_strategy() -> BoxedStrategy<Vec<u8>> {
    proptest::collection::vec(any::<u8>(), 0..=16).boxed()
}

fn repeat_n_strategy() -> BoxedStrategy<u8> {
    any::<u8>().boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let mut runner = TestRunner::new(ProptestConfig::default());
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "HexUintOverflowErrors" => runner
            .run(&hex_bytes_strategy(), move |args| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_hex_uint_overflow_errors(args.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({:?})", args)))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        "IteratorNoPanicAfterDone" => runner
            .run(&iterator_input_strategy(), move |args| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_iterator_no_panic_after_done(args.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({:?})", args)))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        "RepeatNLoopCheckErrors" => runner
            .run(&repeat_n_strategy(), move |args| {
                c.fetch_add(1, Ordering::Relaxed);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_repeat_n_loop_check_errors(args)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => {
                        Err(TestCaseError::fail(format!("({})", args)))
                    }
                }
            })
            .map_err(|e| match e {
                proptest::test_runner::TestError::Fail(r, _) => r.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ---- quickcheck ----

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct BytesArg(Vec<u8>);

impl std::fmt::Display for BytesArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl quickcheck::Arbitrary for BytesArg {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let len = (<u8 as quickcheck::Arbitrary>::arbitrary(g) as usize) % 17;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(<u8 as quickcheck::Arbitrary>::arbitrary(g));
        }
        BytesArg(v)
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(<Vec<u8> as quickcheck::Arbitrary>::shrink(&self.0).map(BytesArg))
    }
}

fn qc_hex_uint_overflow(input: BytesArg) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_hex_uint_overflow_errors(input.0) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_iterator_no_panic(input: BytesArg) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_iterator_no_panic_after_done(input.0) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn qc_repeat_n_loop_check(n: u8) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_repeat_n_loop_check_errors(n) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let result = match property {
        "HexUintOverflowErrors" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_hex_uint_overflow as fn(BytesArg) -> TestResult),
        "IteratorNoPanicAfterDone" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_iterator_no_panic as fn(BytesArg) -> TestResult),
        "RepeatNLoopCheckErrors" => QuickCheck::new()
            .tests(200)
            .max_tests(2000)
            .max_time(Duration::from_secs(86_400))
            .quicktest(qc_repeat_n_loop_check as fn(u8) -> TestResult),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("aborted: {err:?}")),
        ResultStatus::TimedOut => Err("timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    (status, metrics)
}

// ---- crabcheck ----

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_hex_uint_overflow(input: Vec<u8>) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_hex_uint_overflow_errors(input) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_iterator_no_panic(input: Vec<u8>) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_iterator_no_panic_after_done(input) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn cc_repeat_n_loop_check(n: u8) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_repeat_n_loop_check_errors(n) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cfg = crabcheck_qc::Config { tests: 2_000 };
    let result = match property {
        "HexUintOverflowErrors" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_hex_uint_overflow as fn(Vec<u8>) -> Option<bool>,
        ),
        "IteratorNoPanicAfterDone" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_iterator_no_panic as fn(Vec<u8>) -> Option<bool>,
        ),
        "RepeatNLoopCheckErrors" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_repeat_n_loop_check as fn(u8) -> Option<bool>,
        ),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("aborted: {error}"))
        }
    };
    (status, metrics)
}

// ---- hegel ----

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(200)
        .suppress_health_check(hegel::HealthCheck::all())
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "HexUintOverflowErrors" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let len = tc.draw(hgen::integers::<u8>()) as usize % 17;
                let mut buf: Vec<u8> = Vec::with_capacity(len);
                for _ in 0..len {
                    buf.push(tc.draw(hgen::integers::<u8>()));
                }
                let cex = format!("({:?})", buf);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_hex_uint_overflow_errors(buf.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "IteratorNoPanicAfterDone" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let len = tc.draw(hgen::integers::<u8>()) as usize % 17;
                let mut buf: Vec<u8> = Vec::with_capacity(len);
                for _ in 0..len {
                    buf.push(tc.draw(hgen::integers::<u8>()));
                }
                let cex = format!("({:?})", buf);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_iterator_no_panic_after_done(buf.clone())
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "RepeatNLoopCheckErrors" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let n = tc.draw(hgen::integers::<u8>());
                let cex = format!("({})", n);
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_repeat_n_loop_check_errors(n)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{property}"),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

// ---- dispatch + main ----

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!("Properties: HexUintOverflowErrors | IteratorNoPanicAfterDone | RepeatNLoopCheckErrors | All");
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&format!("adapter panic: {msg}")),
            );
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(msg) => emit_json(tool, property, "failed", metrics, Some(&msg), None),
    }
}
