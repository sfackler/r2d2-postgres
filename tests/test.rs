extern crate postgres;
extern crate r2d2;
extern crate r2d2_postgres;

use std::default::Default;
use std::sync::{Arc, Future};
use std::comm;

use postgres::SslMode;
use r2d2_postgres::PostgresPoolManager;
use r2d2_postgres::GenericConnection;

#[test]
fn test_basic() {
    let manager = PostgresPoolManager::new("postgres://postgres@localhost", SslMode::None);
    let config = r2d2::Config {
        pool_size: 2,
        ..Default::default()
    };
    let handler = r2d2::NoopErrorHandler;
    let pool = Arc::new(r2d2::Pool::new(config, manager, handler).unwrap());

    let (s1, r1) = comm::channel();
    let (s2, r2) = comm::channel();

    let pool1 = pool.clone();
    let mut fut1 = Future::spawn(move || {
        let conn = pool1.get().unwrap();
        s1.send(());
        r2.recv();
        drop(conn);
    });

    let pool2 = pool.clone();
    let mut fut2 = Future::spawn(move || {
        let conn = pool2.get().unwrap();
        s2.send(());
        r1.recv();
        drop(conn);
    });

    fut1.get();
    fut2.get();

    pool.get().unwrap();
}

#[test]
fn test_is_valid() {
    let manager = PostgresPoolManager::new("postgres://postgres@localhost", SslMode::None);
    let config = r2d2::Config {
        pool_size: 1,
        test_on_check_out: true,
        ..Default::default()
    };
    let handler = r2d2::NoopErrorHandler;
    let pool = r2d2::Pool::new(config, manager, handler).unwrap();

    pool.get().unwrap();
}

#[test]
fn test_statement_pool() {
    let config = r2d2_postgres::Config { statement_pool_size: 1 };
    let manager = r2d2_postgres::StatementCachingManager::new(
        "postgres://postgres@localhost", SslMode::None, config);
    let pool = r2d2::Pool::new(Default::default(), manager, r2d2::NoopErrorHandler).unwrap();

    let conn = pool.get().unwrap();
    let stmt = conn.prepare("SELECT 1::INT").unwrap();
    let stmt2 = conn.prepare("SELECT 1::INT").unwrap();
    assert_eq!(&*stmt as *const _, &*stmt2 as *const _);
    assert_eq!(stmt.query(&[]).unwrap().next().unwrap().get::<_, i32>(0), 1i32);

    let stmt3 = conn.prepare("SELECT 2::INT").unwrap();
    assert_eq!(stmt3.query(&[]).unwrap().next().unwrap().get::<_, i32>(0), 2i32);
    let stmt4 = conn.prepare("SELECT 1::INT").unwrap();
    let a = &*stmt as *const _;
    let b = &*stmt4 as *const _;
    assert!(a != b);
    assert_eq!(stmt4.query(&[]).unwrap().next().unwrap().get::<_, i32>(0), 1i32);
}
