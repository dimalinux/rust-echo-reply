use log::info;
use tokio::select;
use tokio::signal::unix::{signal, SignalKind};
use tokio_util::sync::CancellationToken;

#[cfg(not(windows))]
pub fn run_signal_handler() -> CancellationToken {
    let run_state = CancellationToken::new();
    let run_state_clone = run_state.clone();

    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sighup = signal(SignalKind::hangup()).unwrap();

    tokio::spawn(async move {
        loop {
            select! {
                biased;
                _ = sigterm.recv() => {
                    run_state.cancel();
                    break;
                },
                _ = sigint.recv() => {
                    run_state.cancel();
                    break;
                },
                _ = sighup.recv() => {
                    info!("Ignoring SIGHUP");
                }
            }
        }
    });

    run_state_clone.clone()
}

#[cfg(windows)]
pub fn run_signal_handler() -> CancellationToken {
    CancellationToken::new().clone()
}

#[cfg(test)]
mod tests {
    use crate::signal_handler::run_signal_handler;
    use tokio::time::sleep;
    #[cfg(not(windows))]
    #[tokio::test]
    async fn test_run_signal_handler() {
        let token = run_signal_handler();
        let pid = std::process::id();

        let result = unsafe { libc::kill(pid as i32, libc::SIGHUP) };
        assert_eq!(result, 0);
        // wait for signal handler to process the SIGHUP
        sleep(tokio::time::Duration::from_millis(100)).await;
        assert!(!token.is_cancelled());

        let result = unsafe { libc::kill(pid as i32, libc::SIGINT) };
        assert_eq!(result, 0);
        token.cancelled().await;
        assert!(token.is_cancelled());
    }
}
