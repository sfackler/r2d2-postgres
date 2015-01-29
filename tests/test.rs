#![allow(unstable)]
extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::default::Default;
use std::sync::{Arc, Future};
use std::sync::mpsc;

use postgres::SslMode;
use r2d2_postgres::PostgresConnectionManager;

#[test]
fn test_basic() {
    let manager = PostgresConnectionManager::new("postgres://postgres@localhost", SslMode::None);
    let config = r2d2::Config {
        pool_size: 2,
        ..Default::default()
    };
    let handler = Box::new(r2d2::NoopErrorHandler);
    let pool = Arc::new(r2d2::Pool::new(config, manager, handler).unwrap());

    let (s1, r1) = mpsc::channel();
    let (s2, r2) = mpsc::channel();

    let pool1 = pool.clone();
    let mut fut1 = Future::spawn(move || {
        let conn = pool1.get().unwrap();
        s1.send(()).unwrap();
        r2.recv().unwrap();
        drop(conn);
    });

    let pool2 = pool.clone();
    let mut fut2 = Future::spawn(move || {
        let conn = pool2.get().unwrap();
        s2.send(()).unwrap();
        r1.recv().unwrap();
        drop(conn);
    });

    fut1.get();
    fut2.get();

    pool.get().unwrap();
}

#[test]
fn test_is_valid() {
    let manager = PostgresConnectionManager::new("postgres://postgres@localhost", SslMode::None);
    let config = r2d2::Config {
        pool_size: 1,
        test_on_check_out: true,
        ..Default::default()
    };
    let handler = Box::new(r2d2::NoopErrorHandler);
    let pool = r2d2::Pool::new(config, manager, handler).unwrap();

    pool.get().unwrap();
}
