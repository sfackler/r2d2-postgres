//! Postgres support for the `r2d2` connection pool.
#![doc(html_root_url="https://sfackler.github.io/doc")]
#![warn(missing_docs)]
#![allow(unstable)]
extern crate r2d2;
extern crate postgres;

use std::error;
use std::error::Error as _StdError;
use std::fmt;
use postgres::{IntoConnectParams, SslMode};

/// A unified enum of errors returned by postgres::Connection
#[derive(Clone, Debug)]
pub enum Error {
    /// A postgres::ConnectError
    Connect(postgres::ConnectError),
    /// An postgres::Error
    Other(postgres::Error),
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

/// An `r2d2::ConnectionManager` for `postgres::Connection`s.
///
/// ## Example
///
/// ```rust,no_run
/// #![allow(unstable)]
/// extern crate r2d2;
/// extern crate r2d2_postgres;
/// extern crate postgres;
///
/// use std::sync::Arc;
/// use std::default::Default;
/// use std::thread::Thread;
/// use postgres::SslMode;
/// use r2d2_postgres::PostgresConnectionManager;
///
/// fn main() {
///     let config = Default::default();
///     let manager = PostgresConnectionManager::new("postgres://postgres@localhost",
///                                                  SslMode::None);
///     let error_handler = r2d2::LoggingErrorHandler;
///     let pool = Arc::new(r2d2::Pool::new(config, manager, error_handler).unwrap());
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         Thread::spawn(move || {
///             let conn = pool.get().unwrap();
///             conn.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
///         });
///     }
/// }
/// ```
pub struct PostgresConnectionManager {
    params: Result<postgres::ConnectParams, postgres::ConnectError>,
    ssl_mode: SslMode,
}

impl fmt::Debug for PostgresConnectionManager {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "PostgresConnectionManager {{ parameters: {:?}, ssl_mode: {:?} }}",
               self.params, self.ssl_mode)
    }
}

impl PostgresConnectionManager {
    /// Creates a new `PostgresConnectionManager`.
    ///
    /// See `postgres::Connection::connect` for a description of the parameter
    /// types.
    pub fn new<T: IntoConnectParams>(params: T, ssl_mode: SslMode) -> PostgresConnectionManager {
        PostgresConnectionManager {
            params: params.into_connect_params(),
            ssl_mode: ssl_mode,
        }
    }
}

impl r2d2::ConnectionManager for PostgresConnectionManager {
    type Connection = postgres::Connection;
    type Error = Error;

    fn connect(&self) -> Result<postgres::Connection, Error> {
        match self.params {
            Ok(ref p) => {
                postgres::Connection::connect(p.clone(), &self.ssl_mode).map_err(Error::Connect)
            }
            Err(ref e) => Err(Error::Connect(e.clone()))
        }
    }

    fn is_valid(&self, conn: &mut postgres::Connection) -> Result<(), Error> {
        conn.batch_execute("").map_err(Error::Other)
    }

    fn has_broken(&self, conn: &mut postgres::Connection) -> bool {
        conn.is_desynchronized()
    }
}
