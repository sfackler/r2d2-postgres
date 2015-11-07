//! Postgres support for the `r2d2` connection pool.
#![doc(html_root_url="https://sfackler.github.io/r2d2-postgres/doc/v0.9.3")]
#![warn(missing_docs)]
extern crate r2d2;
extern crate postgres;

use std::error;
use std::error::Error as _StdError;
use std::fmt;
use postgres::{IntoConnectParams, SslMode};

/// A unified enum of errors returned by postgres::Connection
#[derive(Debug)]
pub enum Error {
    /// A postgres::error::ConnectError
    Connect(postgres::error::ConnectError),
    /// An postgres::error::Error
    Other(postgres::error::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}: {}", self.description(), self.cause().unwrap())
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Connect(_) => "Error opening a connection",
            Error::Other(_) => "Error communicating with server",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Connect(ref err) => Some(err as &error::Error),
            Error::Other(ref err) => Some(err as &error::Error),
        }
    }
}

/// An `r2d2::ManageConnection` for `postgres::Connection`s.
///
/// ## Example
///
/// ```rust,no_run
/// #![allow(unstable)]
/// extern crate r2d2;
/// extern crate r2d2_postgres;
/// extern crate postgres;
///
/// use std::thread;
/// use postgres::SslMode;
/// use r2d2_postgres::PostgresConnectionManager;
///
/// fn main() {
///     let config = r2d2::Config::default();
///     let manager = PostgresConnectionManager::new("postgres://postgres@localhost",
///                                                  SslMode::None).unwrap();
///     let pool = r2d2::Pool::new(config, manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let conn = pool.get().unwrap();
///             conn.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
///         });
///     }
/// }
/// ```
#[derive(Debug)]
pub struct PostgresConnectionManager {
    params: postgres::ConnectParams,
    ssl_mode: SslMode,
}

impl PostgresConnectionManager {
    /// Creates a new `PostgresConnectionManager`.
    ///
    /// See `postgres::Connection::connect` for a description of the parameter
    /// types.
    pub fn new<T: IntoConnectParams>(params: T, ssl_mode: SslMode)
            -> Result<PostgresConnectionManager, postgres::error::ConnectError> {
        let params = match params.into_connect_params() {
            Ok(params) => params,
            Err(err) => return Err(postgres::error::ConnectError::BadConnectParams(err)),
        };

        Ok(PostgresConnectionManager {
            params: params,
            ssl_mode: ssl_mode,
        })
    }
}

impl r2d2::ManageConnection for PostgresConnectionManager {
    type Connection = postgres::Connection;
    type Error = Error;

    fn connect(&self) -> Result<postgres::Connection, Error> {
        postgres::Connection::connect(self.params.clone(), &self.ssl_mode).map_err(Error::Connect)
    }

    fn is_valid(&self, conn: &mut postgres::Connection) -> Result<(), Error> {
        conn.batch_execute("").map_err(Error::Other)
    }

    fn has_broken(&self, conn: &mut postgres::Connection) -> bool {
        conn.is_desynchronized()
    }
}
