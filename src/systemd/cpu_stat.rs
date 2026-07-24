use std::{collections::HashMap, fs, io, time::Instant};

#[derive(Debug, Clone, Copy)]
pub struct CpuStat {
    pub usage_usec: u64,
    pub throttled_usec: u64,
}

pub fn get_cpu_stat(cgroup_path: &str) -> io::Result<CpuStat> {
    let stat_path = format!("/sys/fs/cgroup{cgroup_path}/cpu.stat");
    let content = fs::read_to_string(&stat_path)?;
    parse_cpu_stat(&content)
        .map_err(|err| io::Error::new(err.kind(), format!("{stat_path}: {err}")))
}

/// `throttled_usec` may be absent when the bandwidth controller never ran;
/// treat it as zero. `usage_usec` is mandatory.
fn parse_cpu_stat(content: &str) -> io::Result<CpuStat> {
    let mut usage_usec = None;
    let mut throttled_usec = 0;

    for line in content.lines() {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "usage_usec" => {
                usage_usec = Some(
                    value
                        .trim()
                        .parse::<u64>()
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
                );
            }
            "throttled_usec" => {
                throttled_usec = value
                    .trim()
                    .parse::<u64>()
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
            }
            _ => {}
        }
    }

    Ok(CpuStat {
        usage_usec: usage_usec.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "usage_usec not found in cpu.stat",
            )
        })?,
        throttled_usec,
    })
}

/// Cumulative counters summed over an app's related cgroups.
/// `throttled_usec` stays per cgroup so deltas can take max/any instead of a
/// sum that would under-report a single throttled unit.
#[derive(Debug, Clone)]
pub struct CpuSample {
    pub at: Instant,
    pub usage_usec: u64,
    pub throttled_usec: HashMap<String, u64>,
}

impl CpuSample {
    /// Core-equivalents (1.0 = one fully busy core) since `prev`:
    /// `(summed usage delta / dt, max per-cgroup throttle delta / dt)`.
    /// `None` on non-positive elapsed time.
    pub fn delta_since(&self, prev: &CpuSample) -> Option<(f64, f64)> {
        let dt_usec = self.at.checked_duration_since(prev.at)?.as_micros() as f64;
        if dt_usec <= 0.0 {
            return None;
        }

        let usage = self.usage_usec.saturating_sub(prev.usage_usec) as f64 / dt_usec;
        let throttle = self
            .throttled_usec
            .iter()
            .filter_map(|(cgroup, now)| {
                prev.throttled_usec
                    .get(cgroup)
                    .map(|then| now.saturating_sub(*then))
            })
            .max()
            .unwrap_or(0) as f64
            / dt_usec;

        Some((usage, throttle))
    }
}

/// Sum `cpu.stat` over all related app cgroups.
pub fn sample_cgroups(cgroups: &[String]) -> io::Result<CpuSample> {
    let mut usage_usec = 0;
    let mut throttled_usec = HashMap::new();
    let mut sampled = false;

    for cgroup in cgroups {
        // A unit may be gone mid-read; skip it rather than failing the tick.
        let Ok(stat) = get_cpu_stat(cgroup) else {
            continue;
        };
        usage_usec += stat.usage_usec;
        throttled_usec.insert(cgroup.clone(), stat.throttled_usec);
        sampled = true;
    }

    if !sampled {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no readable cpu.stat for cgroups={cgroups:?}"),
        ));
    }

    Ok(CpuSample {
        at: Instant::now(),
        usage_usec,
        throttled_usec,
    })
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn sample(at: Instant, usage_usec: u64, throttled: &[(&str, u64)]) -> CpuSample {
        CpuSample {
            at,
            usage_usec,
            throttled_usec: throttled
                .iter()
                .map(|(path, value)| (path.to_string(), *value))
                .collect(),
        }
    }

    #[test]
    fn parses_usage_and_throttled() {
        let stat = parse_cpu_stat(
            "usage_usec 123456\nuser_usec 100000\nsystem_usec 23456\nnr_periods 4\nnr_throttled 2\nthrottled_usec 789\n",
        )
        .unwrap();
        assert_eq!(stat.usage_usec, 123456);
        assert_eq!(stat.throttled_usec, 789);
    }

    #[test]
    fn missing_throttled_defaults_to_zero() {
        let stat = parse_cpu_stat("usage_usec 42\nuser_usec 40\n").unwrap();
        assert_eq!(stat.usage_usec, 42);
        assert_eq!(stat.throttled_usec, 0);
    }

    #[test]
    fn missing_usage_is_an_error() {
        let err = parse_cpu_stat("user_usec 40\nthrottled_usec 3\n").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn delta_is_core_equivalents() {
        let start = Instant::now();
        // 5_000_000 usage usec over 10s = 0.5 cores.
        let prev = sample(start, 1_000_000, &[("a", 0)]);
        let next = sample(start + Duration::from_secs(10), 6_000_000, &[("a", 0)]);
        let (usage, throttle) = next.delta_since(&prev).unwrap();
        assert!((usage - 0.5).abs() < 1e-9);
        assert_eq!(throttle, 0.0);
    }

    #[test]
    fn delta_takes_max_throttle_over_cgroups() {
        let start = Instant::now();
        let prev = sample(start, 0, &[("a", 0), ("b", 0)]);
        // "a": 0.1 cores throttled, "b": 0.02 -> max is 0.1.
        let next = sample(
            start + Duration::from_secs(10),
            0,
            &[("a", 1_000_000), ("b", 200_000)],
        );
        let (_, throttle) = next.delta_since(&prev).unwrap();
        assert!((throttle - 0.1).abs() < 1e-9);
    }

    #[test]
    fn delta_ignores_cgroups_missing_from_previous_sample() {
        let start = Instant::now();
        let prev = sample(start, 0, &[("a", 0)]);
        let next = sample(
            start + Duration::from_secs(10),
            0,
            &[("a", 0), ("b", 9_000_000)],
        );
        let (_, throttle) = next.delta_since(&prev).unwrap();
        assert_eq!(throttle, 0.0);
    }

    #[test]
    fn delta_is_none_without_positive_elapsed_time() {
        let at = Instant::now();
        let prev = sample(at, 0, &[]);
        let next = sample(at, 1_000_000, &[]);
        assert!(next.delta_since(&prev).is_none());
    }
}
