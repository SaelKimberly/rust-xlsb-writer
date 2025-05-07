use std::{ffi::OsStr, io::Write, path::Path, sync::Arc};

use rusqlite::{fallible_iterator::FallibleIterator, prepare_cached_and_bind};
use xxhash_rust::xxh3::xxh3_64;
use zerocopy::IntoBytes;

use crate::{KnownID, write_as_biff, write_as_empty_biff};

static PUSH_STMT: &str = "INSERT INTO sst (idx, txt) VALUES (:hash, :data) ON CONFLICT (idx) DO UPDATE SET cnt = cnt + 1;";

#[derive(thiserror::Error, Debug)]
pub enum SSTHolderError {
    #[error("SSTHolder backend database initialization error: {0}")]
    FailureOnInit(rusqlite::Error),
    #[error("SSTHolder backend database failed to set WAL mode")]
    BackendWALError,
    #[error("SSTHolder push failed: {0}")]
    PushFailure(#[from] rusqlite::Error),
    #[error("SSTHolder write to biff sst failed: {0}")]
    BiffWriteIoError(#[from] std::io::Error),
    #[error("SSTHolder write to biff sst failed: {0}")]
    BiffWriteError(rusqlite::Error),
}

/// # SSTHolder
///
/// A wrapper around SQLite database to store shared strings
///
/// Finalized SSTHolder cannot be modified. Finalize the SSTHolder to unlock retrieving indexes.
///
/// Strings in finalized SSTHolder are sorted by their frequency, descending.
///
/// # Usage
///
/// ```
/// # use rust_xlsb_core::SSTHolder;
///
/// let holder = SSTHolder::create_in_memory().unwrap();
///
/// let hash: u64 = holder.push("foo").unwrap();
/// // ... Add more strings
/// holder.finalize().unwrap();
/// let sst: u32 = holder.load(hash).unwrap().unwrap();
///
/// assert_eq!(sst, 0);
/// ```
#[derive(Clone, Debug)]
pub struct SSTHolder {
    conn: Arc<rusqlite::Connection>,
}

impl SSTHolder {
    /// Create a new `SSTHolder`
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::FailureOnInit` - Failed to initialize the database
    /// * `SSTHolderError::BackendWALError` - Failed to set WAL mode
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create(Path::new("test.db")).unwrap();
    /// ```
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn create<S: AsRef<OsStr> + ?Sized>(path: &S) -> Result<Self, SSTHolderError> {
        let path = Path::new(path);
        let conn =
            Arc::new(rusqlite::Connection::open(path).map_err(SSTHolderError::FailureOnInit)?);

        if !conn
            .pragma_update_and_check(None, "journal_mode", "WAL", |row| {
                row.get(0).map(|val: String| val.as_str() == "wal")
            })
            .map_err(SSTHolderError::FailureOnInit)?
        {
            return Err(SSTHolderError::BackendWALError);
        }
        conn.pragma_update(None, "locking_mode", "NORMAL")
            .map_err(SSTHolderError::FailureOnInit)?;
        conn.pragma_update(None, "synchronous", "NORMAL")
            .map_err(SSTHolderError::FailureOnInit)?;
        conn.pragma_update(None, "temp_store", "MEMORY")
            .map_err(SSTHolderError::FailureOnInit)?;

        Self { conn }
            .initialize()
            .map_err(SSTHolderError::FailureOnInit)
    }

    /// Create a new `SSTHolder` in memory. Useful for testing.
    /// When using in-memory database, be careful about memory usage.
    /// # Errors
    ///
    /// * `SSTHolderError::FailureOnInit` - Failed to initialize the database
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// ```
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn create_in_memory() -> Result<Self, SSTHolderError> {
        let conn = Arc::new(
            rusqlite::Connection::open_in_memory().map_err(SSTHolderError::FailureOnInit)?,
        );

        Self { conn }
            .initialize()
            .map_err(SSTHolderError::FailureOnInit)
    }

    /// Push a string to the SSTHolder
    /// Returns the hash of the string
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::PushError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let sst = holder.push("foo").unwrap();
    ///
    /// assert_eq!(sst, 0);
    /// ```
    pub fn push(&self, data: &str) -> Result<u32, SSTHolderError> {
        let hash: i64 = zerocopy::transmute!(xxh3_64(data.as_bytes()));

        Ok(prepare_cached_and_bind!(
            self.conn,
            "INSERT INTO sst (idx, txt) VALUES (:hash, :data) ON CONFLICT (idx) DO UPDATE SET cnt = cnt + 1 RETURNING ROWID - 1;"
        ).raw_query().map(|row| row.get::<_, i64>(0)).next()?.expect("Cannot be null") as u32)
    }

    /// Push multiple strings to the SSTHolder
    /// Returns the hashes of the strings
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::PushError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let idxs = holder.push_batch(["foo", "bar", "bar"]).unwrap();
    ///
    /// assert_eq!(idxs, [0, 1, 1]);
    ///
    /// ```
    pub fn push_batch<S: AsRef<str>>(
        &self,
        data: impl IntoIterator<Item = S>,
    ) -> Result<Vec<u32>, SSTHolderError> {
        self.conn.execute_batch("BEGIN IMMEDIATE TRANSACTION;")?;

        let mut stmt = self.conn.prepare_cached("INSERT INTO sst (idx, txt) VALUES (:hash, :data) ON CONFLICT (idx) DO UPDATE SET cnt = cnt + 1 RETURNING ROWID - 1;")?;

        let ret = data
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .map(|s| {
                let hash: i64 = zerocopy::transmute!(xxh3_64(s.as_bytes()));

                stmt.raw_bind_parameter(c":hash", hash)
                    .and(stmt.raw_bind_parameter(c":data", s))
                    .and(stmt.raw_query().map(|row| row.get::<_, i64>(0)).next())
                    .map(|idx| idx.expect("Cannot be null") as u32)
            })
            .collect::<Result<Vec<u32>, _>>()?;

        self.conn.execute_batch("COMMIT;")?;
        Ok(ret)
    }

    /// Initialize the database
    fn initialize(self) -> rusqlite::Result<Self> {
        self.conn.execute_batch(
            "
            BEGIN;
            DROP TABLE IF EXISTS sst;
            CREATE TABLE IF NOT EXISTS sst (
                idx integer not null unique,
                txt text not null,
                cnt integer not null default 1
            );
            COMMIT;
            VACUUM;
        ",
        )?;
        let _ = self.conn.prepare_cached(PUSH_STMT)?;
        Ok(self)
    }

    /// Write the SSTHolder to a BIFF file
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::BiffWriteError`
    /// * `SSTHolderError::BiffWriteIoError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let idxs = holder.push_batch(["foo", "bar", "bar"]).unwrap();
    ///
    /// assert_eq!(idxs, vec![0, 1, 1]);
    ///
    /// // then write the holder to a BIFF file
    ///
    /// let mut cur = std::io::Cursor::new(Vec::<u8>::with_capacity(1024));
    /// holder.write_to_biff_sst(&mut cur).unwrap();
    /// ```
    #[allow(non_snake_case)]
    pub fn write_to_biff_sst<W: Write>(&self, writer: &mut W) -> Result<(), SSTHolderError> {
        let cstTotal = self
            .conn
            .query_row("SELECT SUM(cnt) FROM sst;", [], |row| {
                row.get(0).map(|val: i64| val as u32)
            })
            .map_err(SSTHolderError::BiffWriteError)?;
        let cstUnique = self
            .conn
            .query_row("SELECT COUNT(*) FROM sst;", [], |row| {
                row.get::<_, i64>(0).map(|val| val as u32)
            })
            .map_err(SSTHolderError::BiffWriteError)?;

        let max_capacity = self
            .conn
            .query_row("SELECT max(length(txt)) FROM sst;", [], |row| {
                row.get(0).map(|val: i64| val as usize)
            })
            .map_err(SSTHolderError::BiffWriteError)?;

        // multiply by 2, because of utf-16. 4 bytes - length of string, one byte reserved.
        let mut buf = Vec::<u8>::with_capacity(max_capacity * 2 + 5);
        buf.push(0x00);

        write_as_biff(
            KnownID::BrtBeginSst,
            [cstTotal, cstUnique].as_bytes(),
            writer,
        )
        .map_err(SSTHolderError::BiffWriteIoError)?;

        let mut stmt = self.conn.prepare("SELECT txt FROM sst;")?;
        let mut query = stmt.raw_query();
        while let Some(txt) = query
            .next()
            .map_err(SSTHolderError::BiffWriteError)?
            .and_then(|row| row.get::<_, String>(0).ok())
            .as_deref()
        {
            buf.truncate(1);
            buf.extend_from_slice((txt.len() as u32).as_bytes());
            txt.encode_utf16().for_each(|c| {
                buf.extend_from_slice(c.as_bytes());
            });

            write_as_biff(KnownID::BrtSSTItem, buf.as_ref(), writer)
                .map_err(SSTHolderError::BiffWriteIoError)?;
        }

        write_as_empty_biff(KnownID::BrtEndSst, writer)
            .map_err(SSTHolderError::BiffWriteIoError)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Seek};

    use deku::DekuContainerRead;

    use crate::{BiffHead, prelude::structure::SimpleRichStr};

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_sst_sqlite() {
        let holder = SSTHolder::create_in_memory().unwrap();

        let foo_idx = holder.push("foo").unwrap();
        assert_eq!(foo_idx, 0);

        let bar_idx_1 = holder.push("bar").unwrap();
        let bar_idx_2 = holder.push("bar").unwrap();
        assert_eq!(bar_idx_1, 1);
        assert_eq!(bar_idx_1, bar_idx_2);

        let idxs = holder
            .push_batch(vec!["some", "batched", "some", "batched"])
            .unwrap();
        assert_eq!(idxs, vec![2, 3, 2, 3]);

        let mut cursor = Cursor::new(Vec::<u8>::with_capacity(1024));
        holder.write_to_biff_sst(&mut cursor).unwrap();

        cursor.rewind().unwrap();

        while let Ok((head, data)) = BiffHead::read_from_with_data(&mut cursor) {
            let id = KnownID::try_from(head.id()).unwrap();
            match id {
                KnownID::BrtBeginSst => {
                    let data: [u32; 2] = zerocopy::FromBytes::read_from_bytes(
                        data.as_deref().expect("Should be present"),
                    )
                    .expect("Should be ok");

                    assert_eq!(data.len(), 2);
                    let (cstTotal, cstUnique) = (data[0], data[1]);
                    assert_eq!(cstTotal, 7);
                    assert_eq!(cstUnique, 4);

                    println!("BrtBeginSst: ({} total, {} unique)", cstTotal, cstUnique);
                }
                KnownID::BrtSSTItem => {
                    let (_, str) =
                        SimpleRichStr::from_bytes((data.as_deref().expect("Should be present"), 0))
                            .expect("Should be ok");
                    println!(
                        "BrtSSTItem: {:?}",
                        str.to_optional_string().expect("Cannot be null")
                    );
                }
                KnownID::BrtEndSst => println!("BrtEndSst"),
                _ => panic!("Should be unreachable"),
            }
        }
    }
}
