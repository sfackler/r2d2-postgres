#![doc(html_root_url="https://sfackler.github.io/doc")]
extern crate r2d2;
extern crate postgres;

use std::fmt;
use postgres::{PostgresConnection, PostgresConnectParams, IntoConnectParams, SslMode};
use postgres::error::{PostgresConnectError, PostgresError};

pub enum Error {
    ConnectError(PostgresConnectError),
    OtherError(PostgresError),
}

impl fmt::Show for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConnectError(ref e) => write!(fmt, "{}", e),
            OtherError(ref e) => write!(fmt, "{}", e),
        }
    }
}

pub struct PostgresPoolManager {
    params: Result<PostgresConnectParams, PostgresConnectError>,
    ssl_mode: SslMode,
}

impl PostgresPoolManager {
    pub fn new<T: IntoConnectParams>(params: T, ssl_mode: SslMode) -> PostgresPoolManager {
        PostgresPoolManager {
            params: params.into_connect_params(),
            ssl_mode: ssl_mode,
        }
    }
}

impl r2d2::PoolManager<PostgresConnection, Error> for PostgresPoolManager {
    fn connect(&self) -> Result<PostgresConnection, Error> {
        match self.params {
            Ok(ref p) => {
                PostgresConnection::connect(p.clone(), &self.ssl_mode).map_err(ConnectError)
            }
            Err(ref e) => Err(ConnectError(e.clone()))
        }
    }

    fn is_valid(&self, conn: &mut PostgresConnection) -> Result<(), Error> {
        conn.batch_execute("").map_err(OtherError)
    }

    fn has_broken(&self, conn: &mut PostgresConnection) -> bool {
        conn.is_desynchronized()
    }
}
