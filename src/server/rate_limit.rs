use std::time::{Duration, Instant};

pub struct RateLimiter {
    time: Instant,
    max_hits: usize,
    hits: usize,
}

impl RateLimiter {
    pub fn new(max_hits: usize) -> Self {
        Self {
            time: Instant::now(),
            max_hits,
            hits: 0,
        }
    }

    pub fn hit(&mut self) -> bool {
        if self.time.elapsed() >= Duration::from_secs(1) {
            self.hits = 0;
            self.time = Instant::now();
        }

        if self.hits >= self.max_hits {
            return true;
        }

        self.hits += 1;
        false
    }
}
