//! Postgres support for the `r2d2` connection pool.
#![warn(missing_docs)]
pub use postgres;
pub use r2d2;

use postgres::tls::{MakeTlsConnect, TlsConnect};
use postgres::{Client, Config, Error, Socket};
use r2d2::ManageConnection;

/// An `r2d2::ManageConnection` for `postgres::Client`s.
///
/// ## Example
///
/// ```no_run
/// use std::thread;
/// use r2d2_postgres::{postgres::NoTls, PostgresConnectionManager};
///
/// fn main() {
///     let manager = PostgresConnectionManager::new(
///         "host=localhost user=postgres".parse().unwrap(),
///         NoTls,
///     );
///     let pool = r2d2::Pool::new(manager).unwrap();
///
///     for i in 0..10i32 {
///         let pool = pool.clone();
///         thread::spawn(move || {
///             let mut client = pool.get().unwrap();
///             client.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
///         });
///     }
/// }
/// ```
#[derive(Debug)]
pub struct PostgresConnectionManager<T> {
    config: Config,
    tls_connector: T,
}

impl<T> PostgresConnectionManager<T>
where
    T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
    T::TlsConnect: Send,
    T::Stream: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    /// Creates a new `PostgresConnectionManager`.
    pub fn new(config: Config, tls_connector: T) -> PostgresConnectionManager<T> {
        PostgresConnectionManager {
            config,
            tls_connector,
        }
    }
}

impl<T> ManageConnection for PostgresConnectionManager<T>
where
    T: MakeTlsConnect<Socket> + Clone + 'static + Sync + Send,
    T::TlsConnect: Send,
    T::Stream: Send,
    <T::TlsConnect as TlsConnect<Socket>>::Future: Send,
{
    type Connection = Client;
    type Error = Error;

    fn connect(&self) -> Result<Client, Error> {
        self.config.connect(self.tls_connector.clone())
    }

    fn is_valid(&self, client: &mut Client) -> Result<(), Error> {
        client.simple_query("").map(|_| ())
    }

    fn has_broken(&self, client: &mut Client) -> bool {
        client.is_closed()
    }
}
