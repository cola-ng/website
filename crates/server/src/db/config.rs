//! Configuration for setting up database pools
//!
//! - `DATABASE_URL`: The URL of the postgres database to use.
//! - `READ_ONLY_REPLICA_URL`: The URL of an optional postgres read-only replica database.
//! - `DB_DIESEL_POOL_SIZE`: The number of connections of the primary database.
//! - `DB_REPLICA_POOL_SIZE`: The number of connections of the read-only / replica database.
//! - `DB_PRIMARY_MIN_IDLE`: The primary pool will maintain at least this number of connections.
//! - `DB_REPLICA_MIN_IDLE`: The replica pool will maintain at least this number of connections.
//! - `DB_OFFLINE`: If set to `leader` then use the read-only follower as if it was the leader. If
//!   set to `follower` then act as if `READ_ONLY_REPLICA_URL` was unset.
//! - `READ_ONLY_MODE`: If defined (even as empty) then force all connections to be read-only.
//! - `DB_TCP_TIMEOUT_MS`: TCP timeout in milliseconds. See the doc comment for more details.

use diesel::prelude::*;
use diesel::r2d2::{self, CustomizeConnection};

#[derive(Debug, Clone, Copy)]
pub struct ConnectionConfig {
    pub statement_timeout: u64,
    // pub read_only: bool,
}

impl CustomizeConnection<PgConnection, r2d2::Error> for ConnectionConfig {
    fn on_acquire(&self, conn: &mut PgConnection) -> Result<(), r2d2::Error> {
        use diesel::sql_query;

        sql_query(format!(
            "SET statement_timeout = {}",
            self.statement_timeout
        ))
        .execute(conn)
        .map_err(r2d2::Error::QueryError)?;
        // if self.read_only {
        //     sql_query("SET default_transaction_read_only = 't'")
        //         .execute(conn)
        //         .map_err(r2d2::Error::QueryError)?;
        // }
        Ok(())
    }
}
