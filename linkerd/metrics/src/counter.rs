use super::prom::{FmtLabels, FmtMetric, MAX_PRECISE_VALUE};
use std::fmt::{self, Display};
use std::sync::atomic::{AtomicU64, Ordering};

/// A Prometheus counter is represented by a `Wrapping` unsigned 52-bit integer.
///
/// Counters always explicitly wrap to zero when value overflows 2^53.
/// This behavior is dictated by the fact that Prometheus represents counters
/// using 64-bit floating-point numbers, whose mantissa is 52-bit wide.
/// Prometheus' [`rate()`] and [`irate()`] queries handle breaks
/// in monotonicity gracefully  (see also [`resets()`]), so derived metrics
/// are still reliable on overflows.
///
/// [`rate()`]: https://prometheus.io/docs/prometheus/latest/querying/functions/#rate()
/// [`irate()`]: https://prometheus.io/docs/prometheus/latest/querying/functions/#irate()
/// [`resets()`]: https://prometheus.io/docs/prometheus/latest/querying/functions/#resets
#[derive(Debug, Default)]
pub struct Counter(AtomicU64);

// ===== impl Counter =====

impl Counter {
    pub fn incr(&self) {
        self.add(1)
    }

    pub fn add(&self, n: u64) {
        self.0.fetch_add(n, Ordering::Release);
    }

    /// Return current counter value, wrapped to be safe for use with Prometheus.
    pub fn value(&self) -> u64 {
        self.0
            .load(Ordering::Acquire)
            .wrapping_rem(MAX_PRECISE_VALUE + 1)
    }
}

impl Into<u64> for Counter {
    fn into(self) -> u64 {
        self.value()
    }
}

impl From<u64> for Counter {
    fn from(value: u64) -> Self {
        Counter(value.into())
    }
}

impl FmtMetric for Counter {
    const KIND: &'static str = "counter";

    fn fmt_metric<N: Display>(&self, f: &mut fmt::Formatter<'_>, name: N) -> fmt::Result {
        writeln!(f, "{} {}", name, self.value())
    }

    fn fmt_metric_labeled<N, L>(
        &self,
        f: &mut fmt::Formatter<'_>,
        name: N,
        labels: L,
    ) -> fmt::Result
    where
        L: FmtLabels,
        N: Display,
    {
        write!(f, "{}{{", name)?;
        labels.fmt_labels(f)?;
        writeln!(f, "}} {}", self.value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_simple() {
        let cnt = Counter::from(0);
        assert_eq!(cnt.value(), 0);
        cnt.incr();
        assert_eq!(cnt.value(), 1);
        cnt.add(41);
        assert_eq!(cnt.value(), 42);
        cnt.add(0);
        assert_eq!(cnt.value(), 42);
    }

    #[test]
    fn count_wrapping() {
        let cnt = Counter::from(MAX_PRECISE_VALUE - 1);
        assert_eq!(cnt.value(), MAX_PRECISE_VALUE - 1);
        cnt.incr();
        assert_eq!(cnt.value(), MAX_PRECISE_VALUE);
        cnt.incr();
        assert_eq!(cnt.value(), 0);
        cnt.incr();
        assert_eq!(cnt.value(), 1);

        let max = Counter::from(MAX_PRECISE_VALUE);
        assert_eq!(max.value(), MAX_PRECISE_VALUE);

        let over = Counter::from(MAX_PRECISE_VALUE + 1);
        assert_eq!(over.value(), 0);
    }
}
