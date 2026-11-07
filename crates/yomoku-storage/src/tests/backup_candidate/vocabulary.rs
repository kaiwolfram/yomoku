use super::*;

fn prefix() -> (TestDatabase, Connection) {
    let database = TestDatabase::new();
    let mut connection = Connection::open(database.path()).unwrap();
    migrate(&mut connection, MIGRATIONS).unwrap();
    (database, connection)
}

#[test]
fn backup_candidate_vocabulary_rejects_blank_surfaces_in_initial_schema() {
    {
        let version = 1;
        for blank in [" ", "\t\r\n", "\u{85}\u{a0}", "\u{2003}\u{3000}"] {
            let (database, connection) = prefix();
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
