use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

type SignalHandler = extern "C" fn(c_int);

extern "C" {
    fn signal(sig: c_int, handler: SignalHandler) -> SignalHandler;
}

extern "C" fn handle_signal(_sig: c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

pub(crate) fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

pub(crate) fn install_signal_handlers() {
    unsafe {
        signal(15, handle_signal); // SIGTERM
        signal(2, handle_signal); // SIGINT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_not_requested_by_default() {
        // Reset state
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
        assert!(!shutdown_requested());
    }

    #[test]
    fn test_handle_signal_sets_shutdown() {
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
        handle_signal(15);
        assert!(shutdown_requested());
        // Reset
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_install_signal_handlers_does_not_panic() {
        install_signal_handlers();
    }
}
