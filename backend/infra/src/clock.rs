use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub trait Clock: Send + Sync {
    fn now_epoch_secs(&self) -> u64;

    fn now_epoch_millis(&self) -> u64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_epoch_secs(&self) -> u64 {
        self.now_epoch_millis() / 1_000
    }

    fn now_epoch_millis(&self) -> u64 {
        #[expect(
            clippy::disallowed_methods,
            reason = "系统时钟实现是唯一允许访问 SystemTime::now 的基础设施边界"
        )]
        let now = SystemTime::now();
        now.duration_since(UNIX_EPOCH).map_or(0, |duration| {
            u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
        })
    }
}

#[derive(Debug)]
pub struct FixedClock {
    epoch_millis: AtomicU64,
}

impl FixedClock {
    #[must_use]
    pub const fn new(epoch_secs: u64) -> Self {
        Self {
            epoch_millis: AtomicU64::new(epoch_secs.saturating_mul(1_000)),
        }
    }

    #[must_use]
    pub const fn from_millis(epoch_millis: u64) -> Self {
        Self {
            epoch_millis: AtomicU64::new(epoch_millis),
        }
    }

    pub fn advance_secs(&self, delta_secs: u64) {
        self.epoch_millis
            .fetch_add(delta_secs.saturating_mul(1_000), Ordering::SeqCst);
    }

    pub fn set_epoch_secs(&self, epoch_secs: u64) {
        self.epoch_millis
            .store(epoch_secs.saturating_mul(1_000), Ordering::SeqCst);
    }
}

impl Clock for FixedClock {
    fn now_epoch_secs(&self) -> u64 {
        self.epoch_millis.load(Ordering::SeqCst) / 1_000
    }

    fn now_epoch_millis(&self) -> u64 {
        self.epoch_millis.load(Ordering::SeqCst)
    }
}
