use super::*;

fn prefix(version: usize) -> (TestDatabase, Connection) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, &MIGRATIONS[..version]).unwrap();
    (database, connection)
}

#[test]
fn backup_candidate_vocabulary_rejects_blank_surfaces_before_and_after_migration() {
    for version in [6, 9, 16, 17, 20, 23, 24] {
        for blank in [" ", "\t\r\n", "\u{85}\u{a0}", "\u{2003}\u{3000}"] {
            let (database, connection) = prefix(version);
            connection
                .execute(
                    "INSERT INTO vocabulary (surface, status, registered_at_unix_ms)
                     VALUES (?1, 'new', 0)",
                    [blank],
                )
                .unwrap();
            drop(connection);
            check_rejected(&database, version as u32, CandidateError::InvalidData);
        }
    }
}
