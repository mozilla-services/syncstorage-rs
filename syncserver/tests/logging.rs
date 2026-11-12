use syncserver::logging::init_logging;

/// Tests call `init_logging` repeatedly: only the first call installs the global subscriber, and
/// later calls still succeed.
#[test]
fn init_logging_can_run_more_than_once() {
    init_logging(false).unwrap();
    init_logging(true).unwrap();
    tracing::error!("logged after init_logging");
}
