use std::{collections::HashMap, time::Instant};

use crate::cgroup::Cgroup;

pub struct CpuSample {
    sample_time: Instant,

    /// CPU usage sum of all cgroups
    usage_usec: u64,

    /// 5 cgroup having 0.2 throttle
    /// doesn't equal to 1 cgroup having 1 throttle
    throttle_usecs: HashMap<Cgroup, u64>,
}

impl CpuSample {
    pub fn delta_since(&self, prev: &CpuSample) -> Option<(f64, f64)> {
        let delta_t_usec = self
            .sample_time
            .checked_duration_since(prev.sample_time)?
            .as_micros() as f64;

        if delta_t_usec <= 0.0 {
            return None;
        }

        // saturating sub to avoid overflow
        let usage = self.usage_usec.saturating_sub(prev.usage_usec) as f64 / delta_t_usec;

        // subtract for each cgroup, then take the max
        let throttle = self
            .throttle_usecs
            .iter()
            .filter_map(|(cgroup, now)| {
                prev.throttle_usecs
                    .get(cgroup)
                    .map(|prev| now.saturating_sub(*prev))
            })
            .max()
            .unwrap_or(0) as f64
            / delta_t_usec;

        Some((usage, throttle))
    }
}
