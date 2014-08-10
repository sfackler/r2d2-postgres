extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::default::Default;
use std::sync::{Arc, Future};
use std::comm;

use postgres::NoSsl;
use postgres::error::{SocketError, InvalidUrl};
use r2d2_postgres::PostgresPoolManager;

#[test]
fn test_bad_url_deferred() {
    let manager = PostgresPoolManager::new("not a url", NoSsl);
    let config = Default::default();
    match r2d2::Pool::new(config, manager) {
        Err(r2d2::ConnectionError(InvalidUrl(_))) => {}
        Err(err) => fail!("Unexpected error {}", err),
        _ => fail!("Unexpected success"),
    }
}

#[test]
fn test_bad_host_error() {
    let manager = PostgresPoolManager::new("postgres://bogushost", NoSsl);
    let config = Default::default();
    match r2d2::Pool::new(config, manager) {
        Err(r2d2::ConnectionError(SocketError(_))) => {}
        Err(err) => fail!("Unexpected error {}", err),
        _ => fail!("Unexpected success")
    }
}

#[test]
fn test_basic() {
    let manager = PostgresPoolManager::new("postgres://postgres@localhost", NoSsl);
    let config = r2d2::Config {
        initial_size: 2,
        ..Default::default()
    };
    let pool = Arc::new(r2d2::Pool::new(config, manager).unwrap());

    let (s1, r1) = comm::channel();
    let (s2, r2) = comm::channel();

    let pool1 = pool.clone();
    let mut fut1 = Future::spawn(proc() {
        let conn = pool1.get().unwrap();
        s1.send(());
        r2.recv();
        conn.replace();
    });

    let pool2 = pool.clone();
    let mut fut2 = Future::spawn(proc() {
        let conn = pool2.get().unwrap();
        s2.send(());
        r1.recv();
        conn.replace();
    });

    fut1.get();
    fut2.get();

    pool.get().unwrap().replace();
}
