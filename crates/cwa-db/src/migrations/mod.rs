//! Database migrations.

use crate::pool::{DbPool, DbResult, DbError};
use rusqlite_migration::{Migrations, M};

/// SQL schema definition.
const SCHEMA: &str = include_str!("schema.sql");

/// Run all database migrations.
pub fn run_migrations(pool: &DbPool) -> DbResult<()> {
    let migrations = Migrations::new(vec![
        M::up(SCHEMA),
    ]);

    pool.with_conn_mut(|conn| {
        migrations
            .to_latest(conn)
            .map_err(|e| DbError::Migration(e.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations() {
        let pool = DbPool::in_memory().unwrap();
        run_migrations(&pool).unwrap();

        // Verify tables exist
        pool.with_conn(|conn| {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='projects'",
                    [],
                    |row| row.get(0),
                )?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }
}
