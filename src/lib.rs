extern crate r2d2;
extern crate postgres;

use postgres::{PostgresConnection, PostgresConnectParams, IntoConnectParams, SslMode};
use postgres::error::PostgresConnectError;

pub struct PostgresPoolManager {
    params: Result<PostgresConnectParams, PostgresConnectError>,
    ssl_mode: SslMode,
}

impl PostgresPoolManager {
    pub fn new<T: IntoConnectParams>(params: T, ssl_mode: SslMode)
                                     -> PostgresPoolManager {
        PostgresPoolManager {
            params: params.into_connect_params(),
            ssl_mode: ssl_mode,
        }
    }
}

impl r2d2::PoolManager<PostgresConnection, PostgresConnectError> for PostgresPoolManager {
    fn connect(&self) -> Result<PostgresConnection, PostgresConnectError> {
        match self.params {
            Ok(ref p) => PostgresConnection::connect(p.clone(), &self.ssl_mode),
            Err(ref e) => Err(e.clone())
        }
    }

    fn is_valid(&self, conn: &PostgresConnection) -> bool {
        conn.batch_execute("SELECT 1").is_ok()
    }
}
