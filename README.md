r2d2-postgres
=============

Documentation is available at https://sfackler.github.io/doc/r2d2_postgres

# Example

```rust
extern crate r2d2;
extern crate r2d2_postgres;
extern crate postgres;

use std::sync::Arc;
use std::default::Default;
use postgres::SslMode;
use r2d2_postgres::PostgresPoolManager;

fn main() {
    let config = Default::default();
    let manager = PostgresPoolManager::new("postgres://postgres@localhost",
                                           SslMode::None);
    let error_handler = r2d2::LoggingErrorHandler;
    let pool = Arc::new(r2d2::Pool::new(config, manager, error_handler).unwrap());

    for i in range(0, 10i32) {
        let pool = pool.clone();
        spawn(proc() {
            let conn = pool.get().unwrap();
            conn.execute("INSERT INTO foo (bar) VALUES ($1)", &[&i]).unwrap();
        });
    }
}
```
