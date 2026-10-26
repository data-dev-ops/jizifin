use jizifin_store::{Database, LATEST_VERSION};

#[test]
fn in_memory_database_initialization_and_version() {
    let db = Database::in_memory().unwrap();
    let schema_ver: i64 = db.read(|conn| {
        conn.query_row("SELECT version FROM schema_version LIMIT 1", [], |r| r.get(0))
            .map_err(Into::into)
    }).unwrap();
    assert_eq!(schema_ver, LATEST_VERSION);
}

#[test]
fn write_transaction_bumps_data_version_while_meta_does_not() {
    let db = Database::in_memory().unwrap();
    let version = db.version();
    // Migrations ran via db.write, so version started at 1
    assert_eq!(version.current(), 1);

    // write_meta does not bump version
    db.write_meta(|tx| {
        tx.execute("SELECT 1", [])?;
        Ok(())
    }).unwrap();
    assert_eq!(version.current(), 1);

    // write bumps version
    db.write(|tx| {
        tx.execute("SELECT 1", [])?;
        Ok(())
    }).unwrap();
    assert_eq!(version.current(), 2);
}
