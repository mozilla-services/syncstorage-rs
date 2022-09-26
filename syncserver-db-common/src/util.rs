use std::fmt;

use actix_web::{error::BlockingError, web};

pub async fn run_on_blocking_threadpool<F, T, E, M>(f: F, e: M) -> Result<T, E>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: fmt::Debug + Send + 'static,
    M: FnOnce(String) -> E,
{
    web::block(f)
        .await
        .map_err(|blocking_error| match blocking_error {
            BlockingError::Error(inner) => inner,
            BlockingError::Canceled => e("Db threadpool operation canceled".to_owned()),
        })
}

#[macro_export]
macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result, Self::Error> {
            let db = self.clone();
            Box::pin(util::run_on_blocking_threadpool(
                move || db.$sync_name(params),
                Self::Error::internal,
            ))
        }
    };
}
