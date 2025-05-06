use std::{
    ffi::OsStr,
    io::Write,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
};

use xxhash_rust::xxh3::xxh3_64;
use zerocopy::IntoBytes;

use crate::{KnownID, write_as_biff, write_as_empty_biff};

static PUSH_STMT: &str = "INSERT INTO sst (idx, txt) VALUES (:hash, :data) ON CONFLICT (idx) DO UPDATE SET cnt = cnt + 1;";

#[derive(thiserror::Error, Debug)]
pub enum SSTHolderError {
    #[error("SSTHolder backend database initialization error: {0}")]
    FailureOnInit(rusqlite::Error),
    #[error("SSTHolder backend database failed to finalize: {0}")]
    FinalizeError(rusqlite::Error),
    #[error("SSTHolder backend database failed to set WAL mode")]
    BackendWALError,
    #[error("SSTHolder push failed: {0}")]
    PushFailure(#[from] rusqlite::Error),
    #[error("SSTHolder is finalized, and cannot be modified")]
    LockedError,
    #[error("SSTHolder load failed: {0}")]
    LoadFailure(rusqlite::Error),
    #[error("SSTHolder is not finalized yet")]
    ShouldLocks,
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
    lock: Arc<AtomicBool>,
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

        Self {
            conn,
            lock: Arc::new(AtomicBool::new(false)),
        }
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

        Self {
            conn,
            lock: Arc::new(AtomicBool::new(false)),
        }
        .initialize()
        .map_err(SSTHolderError::FailureOnInit)
    }

    /// Finalize the SSTHolder
    /// This will move the data from `sst` table to `sst_finalized` table
    /// Does nothing if already finalized
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::FinalizeError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hash = holder.push("foo").unwrap();
    /// holder.finalize().unwrap();
    ///
    /// // then use the hash to retrieve the index
    ///
    /// let sst: u32 = holder.load(hash).unwrap().unwrap();
    /// assert_eq!(sst, 0);
    /// ```
    pub fn finalize(&self) -> Result<(), SSTHolderError> {
        if self.lock.swap(true, std::sync::atomic::Ordering::SeqCst) {
            Ok(())
        } else {
            self
                .conn
                .execute_batch(
                    "
                    DROP TABLE IF EXISTS sst_finalized;
                    CREATE TABLE sst_finalized AS SELECT idx, txt, cnt FROM sst ORDER BY cnt DESC, idx ASC;
                ",
                )
                .and_then(|_| self.conn.execute_batch("
                    DROP TABLE sst;
                ")).map_err(|e| {
                    self.lock.store(false, std::sync::atomic::Ordering::SeqCst);
                    SSTHolderError::FinalizeError(e)
                })
        }
    }

    /// Check if the SSTHolder is finalized
    pub fn is_finalized(&self) -> bool {
        self.lock.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Push a string to the SSTHolder
    /// Returns the hash of the string
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::LockedError`
    /// * `SSTHolderError::PushError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hash = holder.push("foo").unwrap();
    ///
    /// // ... Add more strings
    /// holder.finalize().unwrap();
    ///
    /// // then use the hash to retrieve the index
    ///
    /// let sst: u32 = holder.load(hash).unwrap().unwrap();
    /// assert_eq!(sst, 0);
    /// ```
    pub fn push(&self, data: &str) -> Result<u64, SSTHolderError> {
        if self.is_finalized() {
            Err(SSTHolderError::LockedError)
        } else {
            let hash: i64 = zerocopy::transmute!(xxh3_64(data.as_bytes()));

            let mut stmt = self.conn.prepare_cached(PUSH_STMT)?;
            stmt.raw_bind_parameter(c":hash", hash)?;
            stmt.raw_bind_parameter(c":data", data)?;
            let _ = stmt.raw_execute()?;

            Ok(zerocopy::transmute!(hash))
        }
    }

    /// Push multiple strings to the SSTHolder
    /// Returns the hashes of the strings
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::LockedError`
    /// * `SSTHolderError::PushError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hashes = holder.push_batch(["foo", "bar", "bar"]).unwrap();
    ///
    /// // ... Add more strings
    /// holder.finalize().unwrap();
    ///
    /// // then use the hash to retrieve the index
    ///
    /// let sst: u32 = holder.load(hashes[0]).unwrap().unwrap();
    /// assert_eq!(sst, 1);
    /// ```
    pub fn push_batch<S: AsRef<str>>(
        &self,
        data: impl IntoIterator<Item = S>,
    ) -> Result<Vec<u64>, SSTHolderError> {
        if self.is_finalized() {
            Err(SSTHolderError::LockedError)
        } else {
            let mut stmt = self.conn.prepare_cached(PUSH_STMT)?;

            let ret = data
                .into_iter()
                .map(|s| s.as_ref().to_owned())
                .map(|s| {
                    let hash: i64 = zerocopy::transmute!(xxh3_64(s.as_bytes()));

                    stmt.raw_bind_parameter(c":hash", hash)
                        .and(stmt.raw_bind_parameter(c":data", s))
                        .and(stmt.raw_execute())
                        .and(Ok(hash))
                })
                .map(|i| i.map(|i| zerocopy::transmute!(i)))
                .collect::<Result<Vec<u64>, _>>()?;
            Ok(ret)
        }
    }

    /// Retrieve the index of a string by its hash
    /// Returns None if the string is not found
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::ShouldLocks`
    /// * `SSTHolderError::LoadFailure`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hash = holder.push("foo").unwrap();
    ///
    /// // ... Add more strings
    /// holder.finalize().unwrap();
    ///
    /// // then use the hash to retrieve the index
    ///
    /// let sst: u32 = holder.load(hash).unwrap().unwrap();
    /// assert_eq!(sst, 0);
    /// ```
    pub fn load(&self, hash: u64) -> Result<Option<u32>, SSTHolderError> {
        if !self.is_finalized() {
            Err(SSTHolderError::ShouldLocks)
        } else {
            let hash: i64 = zerocopy::transmute!(hash);
            self.conn
                .prepare_cached("SELECT ROWID - 1 FROM sst_finalized WHERE idx = :hash")
                .map_err(SSTHolderError::LoadFailure)
                .and_then(move |mut stmt| {
                    stmt.raw_bind_parameter(c":hash", hash)
                        .map_err(SSTHolderError::LoadFailure)
                        .and(Ok(stmt))
                })
                .and_then(move |mut stmt| {
                    let mut rows = stmt.raw_query();
                    if let Some(row) = rows.next().map_err(SSTHolderError::LoadFailure)? {
                        Ok(Some(
                            row.get(0)
                                .map(|col: i64| col as u32)
                                .map_err(SSTHolderError::LoadFailure)?,
                        ))
                    } else {
                        Ok(None)
                    }
                })
        }
    }

    /// Retrieve the index of multiple strings by their hashes
    /// Returns None for strings not found
    ///
    /// # Errors
    ///
    /// * `SSTHolderError::ShouldLocks`
    /// * `SSTHolderError::LoadFailure`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    ///
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hashes = holder.push_batch(["foo", "bar", "bar"]).unwrap();
    ///
    /// // ... Add more strings
    /// holder.finalize().unwrap();
    ///
    /// // then use the hash to retrieve the index
    ///
    /// let sst: Vec<Option<u32>> = holder.load_batch(hashes).unwrap();
    ///
    /// // note: strings are sorted by frequency
    /// assert_eq!(sst, vec![Some(1), Some(0), Some(0)]);
    /// ```
    pub fn load_batch(
        &self,
        hashes: impl IntoIterator<Item = u64>,
    ) -> Result<Vec<Option<u32>>, SSTHolderError> {
        if !self.is_finalized() {
            Err(SSTHolderError::ShouldLocks)
        } else {
            let hashes = hashes
                .into_iter()
                .map(|hash| {
                    let hash: i64 = zerocopy::transmute!(hash);
                    hash
                })
                .collect::<Vec<_>>();

            self.conn
                .execute_batch("CREATE TEMP TABLE IF NOT EXISTS hashes (idx integer);")
                .map_err(SSTHolderError::LoadFailure)?;

            let mut placeholders = "(?),".repeat(hashes.len());
            placeholders.pop();

            let mut stmt = self
                .conn
                .prepare_cached(format!("INSERT INTO hashes VALUES {};", placeholders).as_str())
                .map_err(SSTHolderError::LoadFailure)?;

            stmt.execute(rusqlite::params_from_iter(hashes))
                .map_err(SSTHolderError::LoadFailure)?;

            let mut stmt = self
                .conn
                .prepare_cached(
                    "SELECT case when dst.txt is null then null else dst.ROWID - 1 end
                        FROM hashes src 
                        LEFT OUTER JOIN sst_finalized dst
                        ON src.idx = dst.idx;",
                )
                .map_err(SSTHolderError::LoadFailure)?;

            let ret = stmt
                .query_map([], |row| {
                    row.get(0).map(|i: Option<i64>| i.map(|i| i as u32))
                })
                .map_err(SSTHolderError::LoadFailure)?
                .collect::<Result<Vec<_>, _>>()?;

            self.conn
                .execute_batch("DELETE FROM hashes;")
                .map_err(SSTHolderError::LoadFailure)?;

            Ok(ret)
        }
    }

    /// Initialize the database
    fn initialize(self) -> rusqlite::Result<Self> {
        self.conn.execute_batch(
            "
            BEGIN;
            DROP TABLE IF EXISTS sst;
            DROP TABLE IF EXISTS sst_finalized;
            CREATE TABLE IF NOT EXISTS sst (
                idx integer primary key,
                txt text not null,
                cnt integer not null default 1
            ) WITHOUT ROWID;
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
    /// * `SSTHolderError::ShouldLocks`
    /// * `SSTHolderError::BiffWriteError`
    /// * `SSTHolderError::BiffWriteIoError`
    ///
    /// # Example
    ///
    /// ```
    /// # use rust_xlsb_core::SSTHolder;
    /// let holder = SSTHolder::create_in_memory().unwrap();
    /// let hashes = holder.push_batch(["foo", "bar", "bar"]).unwrap();
    /// holder.finalize().unwrap();
    ///
    /// // then retrieve indexes for BrtCellIsst records, on saving worksheets
    ///
    /// let idxs = holder.load_batch(hashes).unwrap();
    /// assert_eq!(idxs, vec![Some(1), Some(0), Some(0)]);
    ///
    /// // then write the holder to a BIFF file
    ///
    /// let mut cur = std::io::Cursor::new(Vec::<u8>::with_capacity(1024));
    /// holder.write_to_biff_sst(&mut cur).unwrap();
    /// ```
    #[allow(non_snake_case)]
    pub fn write_to_biff_sst<W: Write>(&self, writer: &mut W) -> Result<(), SSTHolderError> {
        if !self.is_finalized() {
            return Err(SSTHolderError::ShouldLocks);
        }
        let cstTotal = self
            .conn
            .query_row("SELECT SUM(cnt) FROM sst_finalized;", [], |row| {
                row.get(0).map(|val: i64| val as u32)
            })
            .map_err(SSTHolderError::BiffWriteError)?;
        let cstUnique = self
            .conn
            .query_row("SELECT COUNT(*) FROM sst_finalized;", [], |row| {
                row.get(0).map(|val: i64| val as u32)
            })
            .map_err(SSTHolderError::BiffWriteError)?;

        let max_capacity = self
            .conn
            .query_row("SELECT max(length(txt)) FROM sst_finalized;", [], |row| {
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

        let mut stmt = self.conn.prepare("SELECT txt FROM sst_finalized;")?;
        let mut query = stmt.raw_query();
        while let Some(txt) = query
            .next()
            .map_err(SSTHolderError::BiffWriteError)?
            .and_then(|row| row.get(0).map(|txt: String| txt).ok())
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
    use std::{
        assert_matches::assert_matches,
        io::{Cursor, Seek},
    };

    use deku::DekuContainerRead;

    use crate::{BiffHead, prelude::structure::SimpleRichStr};

    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_sst_sqlite() {
        let holder = SSTHolder::create_in_memory().unwrap();
        assert_matches!(holder.load(0), Err(SSTHolderError::ShouldLocks));
        let unresolved_foo = holder.push("foo").unwrap();

        let _ = holder.push("bar").unwrap();
        let unresolved_bar = holder.push("bar").unwrap();

        let hashes = holder
            .push_batch(vec!["some", "batched", "some", "batched"])
            .unwrap();

        holder.finalize().expect("Cannot finalize");

        assert_matches!(holder.load(unresolved_foo), Ok(Some(3)));
        assert_matches!(holder.load(unresolved_bar), Ok(Some(0)));

        let a = holder.load_batch(hashes).expect("Should be ok");
        println!("{:?}", a);

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
