use once_cell::sync::OnceCell;
use rand::prelude::*;

pub static DB_PATH: OnceCell<std::path::PathBuf> = OnceCell::new();

#[macro_export]
macro_rules! table {
    ($name:ident => $schema:expr) => {
        pub(super) struct $name;
        impl Table for $name {
            const TABLE: &'static str = include_str!($schema);
        }
    };
}

pub trait Table {
    const TABLE: &'static str;
}

pub fn get<T: Table>() -> rusqlite::Connection {
    let db = get_connection();
    db.execute_batch(T::TABLE).unwrap();
    db
}

pub fn get_connection() -> rusqlite::Connection {
    if cfg!(test) {
        thread_local!(static TEST_DB_ID: String = format!(
            "file:{}?mode=memory&cache=shared",
            thread_rng()
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(10)
                .collect::<String>()
        ));

        return TEST_DB_ID
            .with(|id| {
                let flags = rusqlite::OpenFlags::SQLITE_OPEN_URI
                    | rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;
                rusqlite::Connection::open_with_flags(&id, flags)
            })
            .unwrap();
    }

    rusqlite::Connection::open(&*DB_PATH.get_or_init(|| "videos.db".into())).expect("open db")
}
