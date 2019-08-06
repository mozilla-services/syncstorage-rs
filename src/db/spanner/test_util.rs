use diesel::r2d2::CustomizeConnection;
use google_spanner1::{
    BeginTransactionRequest, Error, ReadOnly, ReadWrite, TransactionOptions, TransactionSelector,
};

use super::manager::SpannerSession;

#[derive(Debug)]
pub struct SpannerTestTransactionCustomizer;

impl CustomizeConnection<SpannerSession, Error> for SpannerTestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut SpannerSession) -> Result<(), Error> {
        //conn.begin_test_transaction().map_err(PoolError::QueryError)
        let transaction = begin_transaction(conn, true)?;
        conn.test_transaction = Some(transaction);
        Ok(())
    }
}

// XXX: consolidate w/ SpannerDb::begin
fn begin_transaction(
    spanner: &SpannerSession,
    for_write: bool,
) -> Result<TransactionSelector, Error> {
    let session = spanner.session.name.as_ref().unwrap();
    let mut options = TransactionOptions::default();
    if for_write {
        options.read_write = Some(ReadWrite::default());
    } else {
        options.read_only = Some(ReadOnly::default());
    }
    let req = BeginTransactionRequest {
        options: Some(options),
    };
    let result = spanner
        .hub
        .projects()
        .instances_databases_sessions_begin_transaction(req, session)
        .doit();
    match result {
        Ok((_, transaction)) => Ok(google_spanner1::TransactionSelector {
            id: transaction.id,
            ..Default::default()
        }),
        Err(e) => {
            // TODO Handle error
            Err(e)
        }
    }
}
