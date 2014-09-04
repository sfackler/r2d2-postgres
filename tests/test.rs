extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::default::Default;
use std::sync::{Arc, Future};
use std::comm;

use postgres::NoSsl;
use postgres::error::{SocketError, InvalidUrl, PostgresConnectError};
use r2d2_postgres::PostgresPoolManager;

#[test]
fn test_bad_url_deferred() {
    let manager = PostgresPoolManager::new("not a url", NoSsl);
    let config = Default::default();
    let handler = r2d2::NoopErrorHandler::<PostgresConnectError>;
    match r2d2::Pool::new(config, manager, handler) {
        Err(r2d2::ConnectionError(InvalidUrl(_))) => {}
        Err(err) => fail!("Unexpected error {}", err),
        _ => fail!("Unexpected success"),
    }
}

#[test]
fn test_bad_host_error() {
    let manager = PostgresPoolManager::new("postgres://bogushost", NoSsl);
    let config = Default::default();
    let handler = r2d2::NoopErrorHandler::<PostgresConnectError>;
    match r2d2::Pool::new(config, manager, handler) {
        Err(r2d2::ConnectionError(SocketError(_))) => {}
        Err(err) => fail!("Unexpected error {}", err),
        _ => fail!("Unexpected success")
    }
}

#[test]
fn test_basic() {
    let manager = PostgresPoolManager::new("postgres://postgres@localhost", NoSsl);
    let config = r2d2::Config {
        pool_size: 2,
        ..Default::default()
    };
    let handler = r2d2::NoopErrorHandler::<PostgresConnectError>;
    let pool = Arc::new(r2d2::Pool::new(config, manager, handler).unwrap());

    let (s1, r1) = comm::channel();
    let (s2, r2) = comm::channel();

    let pool1 = pool.clone();
    let mut fut1 = Future::spawn(proc() {
        let conn = pool1.get().unwrap();
        s1.send(());
        r2.recv();
        drop(conn);
    });

    let pool2 = pool.clone();
    let mut fut2 = Future::spawn(proc() {
        let conn = pool2.get().unwrap();
        s2.send(());
        r1.recv();
        drop(conn);
    });

    fut1.get();
    fut2.get();

    pool.get().unwrap();
}
