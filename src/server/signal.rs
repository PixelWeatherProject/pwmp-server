use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct SignalHandle(Arc<AtomicBool>);

impl SignalHandle {
    pub fn new(signal: i32) -> Self {
        let set_flag = Arc::new(AtomicBool::new(false));

        signal_hook::flag::register(signal, Arc::clone(&set_flag))
            .expect("Failed to set up signal hooks");

        Self(set_flag)
    }

    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn unset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl Clone for SignalHandle {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

unsafe impl Send for SignalHandle {}
unsafe impl Sync for SignalHandle {}
