//! Postgres support for the `r2d2` connection pool.
#![doc(html_root_url="https://docs.rs/r2d2_postgres/0.14")]
#![warn(missing_docs)]
pub extern crate r2d2;
pub extern crate postgres;
extern crate postgres_shared;

use postgres::{Connection, Error, Result};
use postgres::params::{ConnectParams, IntoConnectParams};
use postgres::tls::TlsHandshake;

/// Like `postgres::TlsMode` except that it owns its `TlsHandshake` instance.
#[derive(Debug)]
pub enum TlsMode {
    /// Like `postgres::TlsMode::None`.
    None,
    /// Like `postgres::TlsMode::Prefer`.
    Prefer(Box<TlsHandshake + Sync + Send>),
    /// Like `postgres::TlsMode::Require`.
    Require(Box<TlsHandshake + Sync + Send>),
}

/// An `r2d2::ManageConnection` for `postgres::Connection`s.
///
/// ## Example
///
/// ```rust,no_run
/// extern crate r2d2;
/// extern crate r2d2_postgres;
///
/// use std::thread;
/// use r2d2_postgres::{TlsMode, PostgresConnectionManager};
///
/// fn main() {
///     let manager = PostgresConnectionManager::new("postgres://postgres@localhost",
///                                                  TlsMode::None).unwrap();
///     let pool = r2d2::Pool::new(manager).unwrap();
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
    params: ConnectParams,
    ssl_mode: TlsMode,
}

impl PostgresConnectionManager {
    /// Creates a new `PostgresConnectionManager`.
    ///
    /// See `postgres::Connection::connect` for a description of the parameter
    /// types.
    pub fn new<T>(params: T,
                  ssl_mode: TlsMode)
                  -> Result<PostgresConnectionManager>
        where T: IntoConnectParams
    {
        // FIXME we shouldn't be using this private constructor :(
        let params = params.into_connect_params().map_err(postgres_shared::error::connect)?;

        Ok(PostgresConnectionManager {
            params: params,
            ssl_mode: ssl_mode,
        })
    }
}

impl r2d2::ManageConnection for PostgresConnectionManager {
    type Connection = Connection;
    type Error = Error;

    fn connect(&self) -> Result<postgres::Connection> {
        let mode = match self.ssl_mode {
            TlsMode::None => postgres::TlsMode::None,
            TlsMode::Prefer(ref n) => postgres::TlsMode::Prefer(&**n),
            TlsMode::Require(ref n) => postgres::TlsMode::Require(&**n),
        };
        postgres::Connection::connect(self.params.clone(), mode)
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<()> {
        conn.batch_execute("")
    }

    fn has_broken(&self, conn: &mut Connection) -> bool {
        conn.is_desynchronized()
    }
}
