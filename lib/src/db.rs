extern crate log;

/**
Windows: Register sqlite dll with "lib /MACHINE:X64 /def:sqlite3.def /out:sqlite3.lib" on x64
**/

use ::{schema};

//use util::get_notes_file_path_from_metadata;
use diesel::{SqliteConnection, Connection};
use std::env;
use diesel::*;
use diesel::result::Error;
use model::{NotesMetadata, Body};
use schema::metadata::dsl::metadata;
use self::log::*;

pub fn delete_everything(connection: &SqliteConnection) -> Result<(), Error> {
    connection.transaction::<_,Error,_>(|| {
        diesel::delete(schema::metadata::dsl::metadata)
            .execute(connection)?;

        diesel::delete(schema::body::dsl::body)
            .execute(connection)?;

        Ok(())
    })
}

/// Appends a note to an already present note
///
/// Multiple notes only occur if you altered a note locally
/// and server-side, or if 2 separate devices edited the
/// same note, in that case 2 notes exists on the imap
/// server.
///
pub fn append_note(connection: &SqliteConnection, body: &Body) -> Result<(), Error> {
    connection.transaction::<_,Error,_>(|| {
        diesel::insert_into(schema::body::table)
            .values(body)
            .execute(connection)?;

        Ok(())
    })
}

/// Inserts the provided post into the sqlite db
pub fn insert_into_db(connection: &SqliteConnection, note: (&NotesMetadata, &Body) ) -> Result<(), Error> {
    connection.transaction::<_,Error,_>(|| {
        diesel::insert_into(schema::metadata::table)
            .values(note.0)
            .execute(connection)?;

        diesel::insert_into(schema::body::table)
            .values(note.1)
            .execute(connection)?;

        Ok(())
    })
}

pub fn fetch_single_note(connection: &SqliteConnection, id: String) -> Result<(NotesMetadata, Vec<Body>), Error> {
    let mut notes: Vec<NotesMetadata> = metadata
        .filter(schema::metadata::dsl::uuid.eq(&id))
        .load::<NotesMetadata>(connection)?;

    assert!(&notes.len() >= &1_usize);

    let first_note = notes.remove(0);

    debug!("Fetched note with uuid {}", first_note.uuid.clone());

    let body = ::model::Body::belonging_to(&first_note)
        .load::<Body>(connection)?;

    debug!("This note has {} subnotes ", body.len());

    Ok((first_note,body))
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

/// Should insert a single metadata object with a body
///
/// This test should return this note correctly after it got
/// saved.
#[test]
fn insert_single_note() {
    use util::HeaderBuilder;

    simple_logger::init_with_level(Level::Debug).unwrap();
    let con = establish_connection();
    delete_everything(&con);
    let m_data: ::model::NotesMetadata = NotesMetadata::new(HeaderBuilder::new().build(), "test".to_string());
    let body = Body::new(0, m_data.uuid.clone());

    insert_into_db(&con,(&m_data,&body));

    match fetch_single_note(&con, m_data.uuid.clone()) {
        Ok((note, mut bodies)) => {
            assert_eq!(note,m_data);
            assert_eq!(bodies.len(),1);

            let first_note = bodies.pop().unwrap();
            assert_eq!(first_note,body);

        },
        Err(e) => panic!("Fetch DB Call failed {}", e.to_string())
    }
}

/// Should crash because it inserts multiple notes with the same
/// uuid
#[test]
fn no_duplicate_entries() {
    use util::HeaderBuilder;

    simple_logger::init_with_level(Level::Debug).unwrap();
    let con = establish_connection();
    delete_everything(&con);
    let m_data: ::model::NotesMetadata = NotesMetadata::new(HeaderBuilder::new().build(), "test".to_string());
    let body = Body::new(0, m_data.uuid.clone());

    match insert_into_db(&con,(&m_data,&body))
        .and_then(|_| insert_into_db(&con,(&m_data,&body))) {
        Err(e) => assert_eq!(e.to_string(),"UNIQUE constraint failed: metadata.uuid") ,
        _ => panic!("This insert operation should panic"),
    };
}

/// Appends an additional note to a super-note and checks if both are there
#[test]
fn append_additional_note() {
    use util::HeaderBuilder;
    use dotenv::dotenv;

    dotenv::dotenv().ok();
    simple_logger::init_with_level(Level::Debug).unwrap();
    let con = establish_connection();
    delete_everything(&con);
    let m_data: ::model::NotesMetadata = NotesMetadata::new(HeaderBuilder::new().build(), "test".to_string());
    let body = Body::new(0, m_data.uuid.clone());
    let additional_body = Body::new(1, m_data.uuid.clone());

    match insert_into_db(&con,(&m_data,&body))
        .and_then(|_| append_note(&con, &additional_body))
        .and_then(|_| fetch_single_note(&con, m_data.uuid.clone())) {
        Ok((note, mut bodies)) => {
            assert_eq!(note,m_data);
            assert_eq!(bodies.len(),2);

            let first_note = bodies.pop().unwrap();
            let second_note = bodies.pop().unwrap();

            //TODO check if order is always the same
            assert_eq!(second_note,body);
            assert_eq!(first_note,additional_body);

        },
        Err(e) => panic!("DB Transaction failed: {}", e.to_string())
    }


}