use actix::prelude::*;
use diesel::pg::PgConnection;
use diesel::Connection;

pub struct DbExecutor {
    pub connection: PgConnection,
}

impl Actor for DbExecutor {
    type Context = SyncContext<Self>;
}

/// This is state where we will store *DbExecutor* address.
pub struct AppState {
    pub db: Addr<DbExecutor>,
}

pub fn establish_connection(database_url: &str) -> DbExecutor {
    DbExecutor {
        connection: PgConnection::establish(&database_url).unwrap(),
    }
}
