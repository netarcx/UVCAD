// Database migration utilities
// Future migrations can be added here

use crate::utils::error::Result;
use rusqlite::Connection;

pub struct Migrations;

impl Migrations {
    pub fn run(_conn: &Connection) -> Result<()> {
        // Future migrations will be added here
        // For now, the schema is created in schema.rs
        Ok(())
    }
}
