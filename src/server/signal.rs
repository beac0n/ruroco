use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::os::raw::c_int;
use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

pub(crate) fn shutdown_requested() -> bool {
    SHUTDOWN_REQUESTED.load(Ordering::SeqCst)
}

pub(crate) fn install_signal_handlers() {
    let action =
        SigAction::new(SigHandler::Handler(handle_signal), SaFlags::empty(), SigSet::empty());
    unsafe {
        let _ = signal::sigaction(Signal::SIGTERM, &action);
        let _ = signal::sigaction(Signal::SIGINT, &action);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_not_requested_by_default() {
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
        assert!(!shutdown_requested());
    }

    #[test]
    fn test_handle_signal_sets_shutdown() {
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
        handle_signal(15);
        assert!(shutdown_requested());
        SHUTDOWN_REQUESTED.store(false, Ordering::SeqCst);
    }

    #[test]
    fn test_install_signal_handlers_does_not_panic() {
        install_signal_handlers();
    }
}
