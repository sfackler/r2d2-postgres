//! Postgres support for the `r2d2` connection pool.
#![doc(html_root_url="https://sfackler.github.io/doc")]
#![warn(missing_docs)]
extern crate r2d2;
extern crate postgres;

use std::cell::RefCell;
use std::collections::LruCache;
use std::default::Default;
use std::error;
use std::fmt;
use std::mem;
use std::rc::Rc;
use postgres::{IntoConnectParams, SslMode};
use postgres::types::ToSql;

/// A unified enum of errors returned by postgres::Connection
#[deriving(Clone)]
pub enum Error {
    /// A postgres::ConnectError
    Connect(postgres::ConnectError),
    /// An postgres::Error
    Other(postgres::Error),
}

impl fmt::Show for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Connect(ref e) => write!(fmt, "{}", e),
            Error::Other(ref e) => write!(fmt, "{}", e),
        }
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

/// An `r2d2::PoolManager` for `postgres::Connection`s.
///
/// ## Example
///
/// ```rust,no_run
/// extern crate r2d2;
/// extern crate r2d2_postgres;
/// extern crate postgres;
///
/// use std::sync::Arc;
/// use std::default::Default;
/// use postgres::SslMode;
/// use r2d2_postgres::PostgresPoolManager;
///
/// fn main() {
///     let config = Default::default();
///     let manager = PostgresPoolManager::new("postgres://postgres@localhost",
///                                            SslMode::None);
///     let error_handler = r2d2::LoggingErrorHandler;
///     let pool = Arc::new(r2d2::Pool::new(config, manager, error_handler).unwrap());
///
///     for i in range(0, 10i32) {
///         let pool = pool.clone();
///         spawn(move || {
///             let conn = pool.get().unwrap();
///             conn.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
///         });
///     }
/// }
/// ```
pub struct PostgresPoolManager {
    params: Result<postgres::ConnectParams, postgres::ConnectError>,
    ssl_mode: SslMode,
}

impl PostgresPoolManager {
    /// Creates a new `PostgresPoolManager`.
    ///
    /// See `postgres::Connection::connect` for a description of the parameter
    /// types.
    pub fn new<T: IntoConnectParams>(params: T, ssl_mode: SslMode) -> PostgresPoolManager {
        PostgresPoolManager {
            params: params.into_connect_params(),
            ssl_mode: ssl_mode,
        }
    }
}

impl r2d2::PoolManager<postgres::Connection, Error> for PostgresPoolManager {
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

/// Configuration options for the `CachingStatementManager`.
#[deriving(Copy, Clone)]
pub struct Config {
    /// The number of `postgres::Statement`s that will be internally cached.
    ///
    /// Defaults to 10
    pub statement_pool_size: uint,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            statement_pool_size: 10,
        }
    }
}

/// An `r2d2::PoolManager` for `Connection`s, which cache prepared statements.
pub struct StatementCachingManager {
    manager: PostgresPoolManager,
    config: Config,
}

impl StatementCachingManager {
    /// Creates a new `StatementCachingManager`.
    ///
    /// See `postgres::Connection::Connect` for details of the first two
    /// parameter types.
    pub fn new<T>(params: T, ssl_mode: SslMode, config: Config) -> StatementCachingManager
            where T: IntoConnectParams {
        StatementCachingManager {
            manager: PostgresPoolManager::new(params, ssl_mode),
            config: config
        }
    }
}

impl r2d2::PoolManager<Connection, Error> for StatementCachingManager {
    fn connect(&self) -> Result<Connection, Error> {
        let cache = box RefCell::new(LruCache::<String, postgres::Statement<'static>>::new(
                self.config.statement_pool_size));
        Ok(Connection {
            conn: box try!(self.manager.connect()),
            stmts: unsafe { mem::transmute(cache) },
        })
    }

    fn is_valid(&self, conn: &mut Connection) -> Result<(), Error> {
        self.manager.is_valid(&mut *conn.conn)
    }

    fn has_broken(&self, conn: &mut Connection) -> bool {
        self.manager.has_broken(&mut *conn.conn)
    }
}

/// A trait abstracting over functionality provided by `Connection`s and
/// `Transaction`s.
pub trait GenericConnection {
    /// Like `postgres::Connection::prepare`.
    fn prepare<'a>(&'a self, query: &str) -> postgres::Result<Rc<postgres::Statement<'a>>>;

    /// Like `postgres::Connection::execute`.
    fn execute(&self, query: &str, params: &[&ToSql]) -> postgres::Result<uint> {
        self.prepare(query).and_then(|s| s.execute(params))
    }

    /// Like `postgres::Connection::prepare_copy_in`.
    fn prepare_copy_in<'a>(&'a self, table: &str, columns: &[&str])
                           -> postgres::Result<postgres::CopyInStatement<'a>>;

    /// Like `postgres::Connection::transaction`.
    fn transaction<'a>(&'a self) -> postgres::Result<Transaction<'a>>;

    /// Like `postgres::Connection::batch_execute`.
    fn batch_execute(&self, query: &str) -> postgres::Result<()>;
}

/// Like a `postgres::Connection`, but maintains a cache of
/// `postgres::Statement`s.
pub struct Connection {
    conn: Box<postgres::Connection>,
    stmts: *mut (),
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _: Box<RefCell<LruCache<String, Rc<postgres::Statement<'static>>>>> =
            unsafe { mem::transmute(self.stmts) };
    }
}

impl Connection {
    fn get_cache<'a>(&'a self) -> &'a RefCell<LruCache<String, Rc<postgres::Statement<'a>>>> {
        unsafe { mem::transmute(self.stmts) }
    }
}

impl GenericConnection for Connection {
    fn prepare<'a>(&'a self, query: &str) -> postgres::Result<Rc<postgres::Statement<'a>>> {
        let query = query.into_string();
        let mut stmts = self.get_cache().borrow_mut();

        if let Some(stmt) = stmts.get(&query) {
            return Ok(stmt.clone());
        }

        let stmt = Rc::new(try!(self.conn.prepare(query[])));
        stmts.insert(query, stmt.clone());
        Ok(stmt)
    }

    fn prepare_copy_in<'a>(&'a self, table: &str, columns: &[&str])
                           -> postgres::Result<postgres::CopyInStatement<'a>> {
        self.conn.prepare_copy_in(table, columns)
    }

    fn transaction<'a>(&'a self) -> postgres::Result<Transaction<'a>> {
        Ok(Transaction {
            conn: self,
            trans: try!(self.conn.transaction())
        })
    }

    fn batch_execute(&self, query: &str) -> postgres::Result<()> {
        self.conn.batch_execute(query)
    }
}

/// Like `postgres::Transaction`.
pub struct Transaction<'a> {
    conn: &'a Connection,
    trans: postgres::Transaction<'a>
}

impl<'a> GenericConnection for Transaction<'a> {
    fn prepare<'b>(&'b self, query: &str) -> postgres::Result<Rc<postgres::Statement<'b>>> {
        let query = query.into_string();
        let mut stmts = self.conn.get_cache().borrow_mut();

        if let Some(stmt) = stmts.get(&query) {
            return Ok(stmt.clone());
        }

        Ok(Rc::new(try!(self.trans.prepare(query[]))))
    }

    fn prepare_copy_in<'b>(&'b self, table: &str, columns: &[&str])
                           -> postgres::Result<postgres::CopyInStatement<'b>> {
        self.trans.prepare_copy_in(table, columns)
    }

    fn transaction<'b>(&'b self) -> postgres::Result<Transaction<'b>> {
        Ok(Transaction {
            conn: self.conn,
            trans: try!(self.trans.transaction())
        })
    }

    fn batch_execute(&self, query: &str) -> postgres::Result<()> {
        self.trans.batch_execute(query)
    }
}
