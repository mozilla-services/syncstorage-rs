//! Runs in its own process, so no tracing subscriber is installed until the test does it.
#![cfg(feature = "last_seen")]

use syncserver::{
    logging::{init_logging, row_write_failures},
    tokenserver::last_seen::RowLog,
};

#[test]
fn row_log_requires_init_logging() {
    let err = RowLog::spawn(row_write_failures(), None)
        .err()
        .expect("an error");
    assert_eq!(err.to_string(), "Row logger requires init_logging");

    init_logging(false).unwrap();
    assert!(RowLog::spawn(row_write_failures(), None).is_ok());
}
