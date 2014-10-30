#![doc(html_root_url="https://sfackler.github.io/doc")]
#![feature(if_let)]
extern crate r2d2;
extern crate postgres;

use std::cell::RefCell;
use std::collections::LruCache;
use std::default::Default;
use std::fmt;
use std::mem;
use std::rc::Rc;
use postgres::{PostgresConnection,
               PostgresConnectParams,
               IntoConnectParams,
               SslMode,
               PostgresResult,
               PostgresStatement,
               PostgresCopyInStatement,
               PostgresTransaction};
use postgres::error::{PostgresConnectError, PostgresError};
use postgres::types::ToSql;

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

pub struct Config {
    pub statement_pool_size: uint,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            statement_pool_size: 10,
        }
    }
}

pub struct StatementCachingManager {
    manager: PostgresPoolManager,
    config: Config,
}

impl StatementCachingManager {
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
        let cache = box RefCell::new(LruCache::<String, PostgresStatement<'static>>::new(
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

pub trait GenericConnection {
    /// Like `PostgresConnection::prepare`.
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<Rc<PostgresStatement<'a>>>;

    /// Like `PostgresConnection::execute`.
    fn execute(&self, query: &str, params: &[&ToSql]) -> PostgresResult<uint> {
        self.prepare(query).and_then(|s| s.execute(params))
    }

    /// Like `PostgresConnection::prepare_copy_in`.
    fn prepare_copy_in<'a>(&'a self, table: &str, columns: &[&str])
                           -> PostgresResult<PostgresCopyInStatement<'a>>;

    /// Like `PostgresConnection::transaction`.
    fn transaction<'a>(&'a self) -> PostgresResult<Transaction<'a>>;

    /// Like `PostgresConnection::batch_execute`.
    fn batch_execute(&self, query: &str) -> PostgresResult<()>;
}

pub struct Connection {
    conn: Box<PostgresConnection>,
    stmts: *mut (),
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _: Box<RefCell<LruCache<String, Rc<PostgresStatement<'static>>>>> =
            unsafe { mem::transmute(self.stmts) };
    }
}

impl Connection {
    fn get_cache<'a>(&'a self) -> &'a RefCell<LruCache<String, Rc<PostgresStatement<'a>>>> {
        unsafe { mem::transmute(self.stmts) }
    }
}

impl GenericConnection for Connection {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<Rc<PostgresStatement<'a>>> {
        let query = query.into_string();
        let mut stmts = self.get_cache().borrow_mut();

        if let Some(stmt) = stmts.get(&query) {
            return Ok(stmt.clone());
        }

        let stmt = Rc::new(try!(self.conn.prepare(query[])));
        stmts.put(query, stmt.clone());
        Ok(stmt)
    }

    fn prepare_copy_in<'a>(&'a self, table: &str, columns: &[&str])
                           -> PostgresResult<PostgresCopyInStatement<'a>> {
        self.conn.prepare_copy_in(table, columns)
    }

    fn transaction<'a>(&'a self) -> PostgresResult<Transaction<'a>> {
        Ok(Transaction {
            conn: self,
            trans: try!(self.conn.transaction())
        })
    }

    fn batch_execute(&self, query: &str) -> PostgresResult<()> {
        self.conn.batch_execute(query)
    }
}

pub struct Transaction<'a> {
    conn: &'a Connection,
    trans: PostgresTransaction<'a>
}

impl<'a> GenericConnection for Transaction<'a> {
    fn prepare<'a>(&'a self, query: &str) -> PostgresResult<Rc<PostgresStatement<'a>>> {
        let query = query.into_string();
        let mut stmts = self.conn.get_cache().borrow_mut();

        if let Some(stmt) = stmts.get(&query) {
            return Ok(stmt.clone());
        }

        Ok(Rc::new(try!(self.trans.prepare(query[]))))
    }

    fn prepare_copy_in<'a>(&'a self, table: &str, columns: &[&str])
                           -> PostgresResult<PostgresCopyInStatement<'a>> {
        self.trans.prepare_copy_in(table, columns)
    }

    fn transaction<'a>(&'a self) -> PostgresResult<Transaction<'a>> {
        Ok(Transaction {
            conn: self.conn,
            trans: try!(self.trans.transaction())
        })
    }

    fn batch_execute(&self, query: &str) -> PostgresResult<()> {
        self.trans.batch_execute(query)
    }
}
