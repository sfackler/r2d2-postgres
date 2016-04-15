r2d2-postgres
=============

[![Build Status](https://travis-ci.org/sfackler/r2d2-postgres.svg?branch=master)](https://travis-ci.org/sfackler/r2d2-postgres)

[Documentation](https://sfackler.github.io/r2d2-postgres/doc/v0.10.1/r2d2_postgres)

[rust-postgres](https://github.com/sfackler/rust-postgres) support library for the [r2d2](https://github.com/sfackler/r2d2) connection pool.

# Example

```rust
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;

use std::thread;
use r2d2_postgres::{SslMode, PostgresConnectionManager};

fn main() {
    let config = r2d2::Config::default();
    let manager = PostgresConnectionManager::new("postgres://postgres@localhost",
                                                 SslMode::None).unwrap();
    let pool = r2d2::Pool::new(config, manager).unwrap();

    for i in 0..10i32 {
        let pool = pool.clone();
        thread::spawn(move || {
            let conn = pool.get().unwrap();
            conn.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
        });
    }
}
```
