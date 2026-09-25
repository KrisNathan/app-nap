use crate::{
    config::models::cpu_load_polling::CpuLoadPollingConfig, daemon::models::cpu_sample::CpuSample,
};

#[derive(Clone, Copy, PartialEq)]
pub enum Load {
    Busy,
    Idle,
}

pub struct LoadTracker {
    load: Load,
    queued_load: Load,
    ticks: u64,

    usage: f64,
    throttle: f64,

    last_cpu_sample: Option<CpuSample>,
}

impl LoadTracker {
    pub fn new() -> Self {
        Self {
            load: Load::Busy,
            queued_load: Load::Busy,
            ticks: 0,
            usage: 0.0,
            throttle: 0.0,
            last_cpu_sample: None,
        }
    }

    pub fn on_cpu_usage_tick(
        &mut self,
        new_cpu_sample: CpuSample,
        config: &CpuLoadPollingConfig,
    ) -> Option<Load> {
        let Some(last_cpu_sample) = &self.last_cpu_sample else {
            self.last_cpu_sample = Some(new_cpu_sample);
            return None;
        };

        let (usage, throttle) = new_cpu_sample.delta_since(last_cpu_sample)?;

        self.last_cpu_sample = Some(new_cpu_sample);

        self.usage = usage;
        self.throttle = throttle;

        let candidate = eval_next_load(usage, throttle, config)?;

        if candidate != self.queued_load {
            self.queued_load = candidate;
            self.ticks = 1;
        } else {
            self.ticks += 1;
        }

        let confirm_tick = match self.queued_load {
            Load::Idle => config.idle.confirm_ticks,
            Load::Busy => config.busy.confirm_ticks,
        };

        if self.ticks >= confirm_tick {
            self.ticks = 0;
            if self.load != self.queued_load {
                self.load = self.queued_load;
                return Some(self.load);
            }
        }

        None
    }

    /// Resets last_cpu_sample to None
    pub fn clear_baseline(&mut self) {
        self.last_cpu_sample = None;
    }

    /// Resets last_cpu_sample to None, ticks to 0, and queued_load to current load
    pub fn reset(&mut self) {
        self.last_cpu_sample = None;
        self.ticks = 0;
        self.queued_load = self.load;
    }

    pub fn load(&self) -> Load {
        self.load
    }

    pub fn usage(&self) -> f64 {
        self.usage
    }

    pub fn throttle(&self) -> f64 {
        self.throttle
    }
}

fn eval_next_load(usage: f64, throttle: f64, config: &CpuLoadPollingConfig) -> Option<Load> {
    if usage < config.idle.usage_thres && throttle < config.idle.throttle_thres {
        Some(Load::Idle)
    } else if usage > config.busy.usage_thres || throttle > config.busy.throttle_thres {
        Some(Load::Busy)
    } else {
        None
    }
}
