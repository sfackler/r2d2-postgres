
use r2d2;


use std::sync::mpsc;
use std::thread;

use r2d2_postgres::{TlsMode, PostgresConnectionManager};

#[test]
fn test_basic() {
    let manager =
        PostgresConnectionManager::new("postgres://postgres:password@localhost", TlsMode::None)
            .unwrap();
    let pool = r2d2::Pool::builder().max_size(2).build(manager).unwrap();

    let (s1, r1) = mpsc::channel();
    let (s2, r2) = mpsc::channel();

    let pool1 = pool.clone();
    let t1 = thread::spawn(move || {
        let conn = pool1.get().unwrap();
        s1.send(()).unwrap();
        r2.recv().unwrap();
        drop(conn);
    });

    let pool2 = pool.clone();
    let t2 = thread::spawn(move || {
        let conn = pool2.get().unwrap();
        s2.send(()).unwrap();
        r1.recv().unwrap();
        drop(conn);
    });

    t1.join().unwrap();
    t2.join().unwrap();

    pool.get().unwrap();
}

#[test]
fn test_is_valid() {
    let manager =
        PostgresConnectionManager::new("postgres://postgres:password@localhost", TlsMode::None)
            .unwrap();
    let pool = r2d2::Pool::builder()
        .max_size(1)
        .build(manager)
        .unwrap();

    pool.get().unwrap();
}
