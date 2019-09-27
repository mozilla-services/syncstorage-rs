use diesel::r2d2::CustomizeConnection;
#[cfg(not(feature = "google_grpc"))]
use google_spanner1::Error;
#[cfg(feature = "google_grpc")]
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
