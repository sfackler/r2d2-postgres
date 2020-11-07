r2d2-postgres
=============

[![CircleCI](https://circleci.com/gh/sfackler/r2d2-postgres.svg?style=shield)](https://circleci.com/gh/sfackler/r2d2-postgres)

[Documentation](https://docs.rs/r2d2_postgres)

[rust-postgres](https://github.com/sfackler/rust-postgres) support library for the [r2d2](https://github.com/sfackler/r2d2) connection pool.

# Example

```rust
use std::thread;
use r2d2_postgres::{postgres::NoTls, PostgresConnectionManager};

fn main() {
    let manager = PostgresConnectionManager::new(
        "host=localhost user=postgres".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2::Pool::new(manager).unwrap();

    for i in 0..10i32 {
        let pool = pool.clone();
        thread::spawn(move || {
            let mut client = pool.get().unwrap();
            client.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
        });
    }
}
```
