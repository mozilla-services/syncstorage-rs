use diesel::r2d2::CustomizeConnection;
use grpcio::Error;

use super::manager::SpannerSession;

#[derive(Debug)]
pub struct SpannerTestTransactionCustomizer;

impl CustomizeConnection<SpannerSession, Error> for SpannerTestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut SpannerSession) -> Result<(), Error> {
        conn.use_test_transactions = true;
        Ok(())
    }
}
