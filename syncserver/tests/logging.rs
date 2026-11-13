use syncserver::logging::{init_logging, row_logger};

#[test]
fn row_logger_requires_init_logging() {
    assert_eq!(
        row_logger().unwrap_err().to_string(),
        "Row logger requires init_logging"
    );

    init_logging(false).unwrap();
    assert!(row_logger().is_ok());
}
