use crate::{
    config::model::CpuLoadPollingConfig, daemon::app_state::Load, systemd::cpu_stat::CpuSample,
};

/// Busy/idle hysteresis for one cpu-load-polled app: dual thresholds (dead band) plus
/// a throttle escape hatch, and a TTL so a candidate must persist before it is
/// applied. Cold start is `Busy`; the first sample only sets the baseline.
#[derive(Debug)]
pub struct LoadTracker {
    pub load: Load,
    pub queued_load: Load,
    pub ttl: u32,
    pub usage: f64,
    pub throttle: f64,
    last_sample: Option<CpuSample>,
}

impl LoadTracker {
    pub fn new() -> Self {
        Self {
            load: Load::Busy,
            queued_load: Load::Busy,
            ttl: 0,
            usage: 0.0,
            throttle: 0.0,
            last_sample: None,
        }
    }

    /// Back to cold start: busy until fresh samples exist.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Keep load state but drop the sample baseline (e.g. cgroup set changed).
    pub fn clear_baseline(&mut self) {
        self.last_sample = None;
    }

    /// Feed a sample; returns the new load when the hysteresis applies a change.
    pub fn observe(&mut self, sample: CpuSample, config: &CpuLoadPollingConfig) -> Option<Load> {
        let Some(prev) = &self.last_sample else {
            self.last_sample = Some(sample);
            return None;
        };
        let Some((usage, throttle)) = sample.delta_since(prev) else {
            self.last_sample = Some(sample);
            return None;
        };
        self.usage = usage;
        self.throttle = throttle;
        self.last_sample = Some(sample);

        let candidate = match self.load {
            Load::Busy if usage < config.idle_threshold && throttle < config.throttle_idle_max => {
                Load::Idle
            }
            Load::Idle if usage > config.busy_threshold || throttle > config.throttle_busy => {
                Load::Busy
            }
            load => load,
        };

        if candidate != self.queued_load {
            self.queued_load = candidate;
            self.ttl = match candidate {
                Load::Idle => config.ttl_idle,
                Load::Busy => config.ttl_busy,
            };
            return None;
        }

        if self.queued_load == self.load {
            return None;
        }

        self.ttl = self.ttl.saturating_sub(1);
        if self.ttl > 0 {
            return None;
        }

        self.load = self.queued_load;
        Some(self.load)
    }
}

impl Default for LoadTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::*;

    fn config() -> CpuLoadPollingConfig {
        CpuLoadPollingConfig::default()
    }

    fn sample(at: Instant, usage_delta: u64, throttle_delta: u64) -> CpuSample {
        CpuSample {
            at,
            usage_usec: usage_delta,
            throttled_usec: [("a".to_string(), throttle_delta)].into_iter().collect(),
        }
    }

    const SEC: Duration = Duration::from_secs(10);
    // Core-equivalents at a 10s tick: 0.01 idle-ish, 0.10 dead band, 0.5 busy.
    const LOW: u64 = 100_000;
    const MID: u64 = 1_000_000;
    const HIGH: u64 = 5_000_000;

    /// A tracker already in `Idle`, with its last sample at `t0 + 3*SEC`;
    /// callers continue the timeline from `t0 + 4*SEC`.
    fn idle_tracker(t0: Instant) -> LoadTracker {
        let mut tracker = LoadTracker::new();
        tracker.observe(sample(t0, 0, 0), &config());
        tracker.observe(sample(t0 + SEC, LOW, 0), &config());
        tracker.observe(sample(t0 + 2 * SEC, 2 * LOW, 0), &config());
        tracker.observe(sample(t0 + 3 * SEC, 3 * LOW, 0), &config());
        assert_eq!(tracker.load, Load::Idle);
        tracker
    }

    #[test]
    fn first_sample_is_baseline_only() {
        let mut tracker = LoadTracker::new();
        let t0 = Instant::now();
        assert_eq!(tracker.observe(sample(t0, 0, 0), &config()), None);
        assert_eq!(tracker.load, Load::Busy);
    }

    #[test]
    fn idle_applies_after_ttl() {
        let mut tracker = LoadTracker::new();
        let t0 = Instant::now();
        tracker.observe(sample(t0, 0, 0), &config());
        assert_eq!(tracker.observe(sample(t0 + SEC, LOW, 0), &config()), None);
        assert_eq!(tracker.queued_load, Load::Idle);
        assert_eq!(tracker.ttl, 2);
        assert_eq!(
            tracker.observe(sample(t0 + 2 * SEC, 2 * LOW, 0), &config()),
            None
        );
        assert_eq!(tracker.load, Load::Busy);
        assert_eq!(
            tracker.observe(sample(t0 + 3 * SEC, 3 * LOW, 0), &config()),
            Some(Load::Idle)
        );
    }

    #[test]
    fn busy_applies_after_shorter_ttl() {
        let t0 = Instant::now();
        let mut tracker = idle_tracker(t0);
        assert_eq!(
            tracker.observe(sample(t0 + 4 * SEC, 3 * LOW + HIGH, 0), &config()),
            None
        );
        assert_eq!(tracker.queued_load, Load::Busy);
        assert_eq!(tracker.ttl, 1);
        assert_eq!(
            tracker.observe(sample(t0 + 5 * SEC, 3 * LOW + 2 * HIGH, 0), &config()),
            Some(Load::Busy)
        );
    }

    #[test]
    fn dead_band_holds_current_load() {
        let mut tracker = LoadTracker::new();
        let t0 = Instant::now();
        tracker.observe(sample(t0, 0, 0), &config());
        assert_eq!(tracker.observe(sample(t0 + SEC, MID, 0), &config()), None);
        assert_eq!(tracker.queued_load, Load::Busy);
        assert_eq!(tracker.load, Load::Busy);
    }

    #[test]
    fn spike_cancels_pending_idle() {
        let mut tracker = LoadTracker::new();
        let t0 = Instant::now();
        tracker.observe(sample(t0, 0, 0), &config());
        tracker.observe(sample(t0 + SEC, LOW, 0), &config());
        assert_eq!(tracker.queued_load, Load::Idle);
        assert_eq!(
            tracker.observe(sample(t0 + 2 * SEC, LOW + HIGH, 0), &config()),
            None
        );
        assert_eq!(tracker.queued_load, Load::Busy);
        assert_eq!(tracker.load, Load::Busy);
    }

    #[test]
    fn throttle_blocks_idle() {
        let mut tracker = LoadTracker::new();
        let t0 = Instant::now();
        tracker.observe(sample(t0, 0, 0), &config());
        // Usage is low but throttling is above the idle ceiling (0.02 > 0.01).
        assert_eq!(
            tracker.observe(sample(t0 + SEC, LOW, 200_000), &config()),
            None
        );
        assert_eq!(tracker.queued_load, Load::Busy);
    }

    #[test]
    fn throttle_escape_queues_busy() {
        let t0 = Instant::now();
        let mut tracker = idle_tracker(t0);
        // Usage reads low (capped by quota) but throttling crosses 0.05.
        assert_eq!(
            tracker.observe(sample(t0 + 4 * SEC, 4 * LOW, 600_000), &config()),
            None
        );
        assert_eq!(tracker.queued_load, Load::Busy);
        assert_eq!(
            tracker.observe(sample(t0 + 5 * SEC, 5 * LOW, 1_200_000), &config()),
            Some(Load::Busy)
        );
    }

    #[test]
    fn reset_returns_to_cold_start() {
        let t0 = Instant::now();
        let mut tracker = idle_tracker(t0);
        tracker.reset();
        assert_eq!(tracker.load, Load::Busy);
        assert_eq!(tracker.queued_load, Load::Busy);
        assert_eq!(tracker.observe(sample(t0 + 4 * SEC, 0, 0), &config()), None);
    }
}
