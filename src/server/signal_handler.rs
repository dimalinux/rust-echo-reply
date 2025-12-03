use log::info;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};
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

    run_state_clone
}

#[cfg(windows)]
pub fn run_signal_handler() -> CancellationToken {
    // TODO: implement signal handling for Windows (don't currently have a host to test on)
    // https://docs.rs/tokio/latest/tokio/signal/windows/index.html
    CancellationToken::new().clone()
}

#[cfg(test)]
mod tests {
    use tokio::time::sleep;

    use crate::signal_handler::run_signal_handler;
    #[cfg(not(windows))]
    #[tokio::test]
    #[expect(unsafe_code)]
    #[expect(clippy::cast_possible_wrap)]
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
