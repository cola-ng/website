# Database access

Both servers use Diesel query builders with Horme 0.4 async execution and a bounded deadpool PostgreSQL pool. Rust 1.96 or newer is required.

Use `db::with_conn!` and `horme::prelude::*` for application queries. The callback receives a cloneable `horme::Db` handle: each awaited query borrows a connection and releases it before returning. Keeping the callback alive does not reserve a pool slot. Do not acquire a raw pooled connection around network requests, audio generation, or other business work.

For atomic operations, use `horme::transaction(conn, async |conn| { ... })` and pass that callback's connection to every query in the transaction. The transaction retains one connection until commit or rollback. Avoid external I/O inside it.

The pool bounds connection count and sets acquisition, creation, recycling, and statement timeouts. This limits resource use; slow queries and excessive concurrency can still reach the configured capacity.

To run the regression test against a disposable PostgreSQL instance from the workspace root in PowerShell:

```powershell
$env:TEST_DATABASE_URL = 'postgresql://user@127.0.0.1:5432/test_database'
cargo test -p colang --lib db:: -- --include-ignored
```

The test uses a pool of one connection and checks release between queries while a callback remains alive, concurrent callers, query errors, transaction connection reuse and rollback, and cancellation while waiting for a saturated pool. It does not run application migrations or modify application tables.
