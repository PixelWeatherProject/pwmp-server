use pwmp_client::pwmp_msg::MsgId;
use ring::rand::{SecureRandom, SystemRandom};
use std::sync::{Mutex, MutexGuard};

pub struct RngBuf {
    rng: SystemRandom,
    buf: Mutex<Vec<MsgId>>,
    n: usize,
}

impl RngBuf {
    pub fn new(rng: SystemRandom, length: usize) -> Self {
        let new_self = Self {
            rng,
            buf: Mutex::new(Vec::with_capacity(length)),
            n: length,
        };

        new_self.take_next();
        new_self
    }

    pub fn take_next(&self) -> MsgId {
        let mut guard = self.buf.lock().unwrap();

        if guard.is_empty() {
            self.refill(&mut guard);
        }

        let n = guard.drain(0..1).next().unwrap();

        n
    }

    pub fn touch(&self) {}

    fn refill(&self, guard: &mut MutexGuard<'_, Vec<MsgId>>) {
        for _ in 0..self.n {
            let mut n_bytes = [0; size_of::<MsgId>()];
            self.rng.fill(&mut n_bytes).unwrap();

            let n = MsgId::from_ne_bytes(n_bytes);
            guard.push(n);
        }
    }
}
