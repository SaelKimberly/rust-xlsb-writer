use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Unknown ID: {0}")]
    UnknownId(u16),
    #[error("Too large record body: {0}, max: {1}")]
    TooLargeRecordBody(usize, usize),
    #[error("Too large record id: {0}, max: {1}")]
    TooLargeRecordId(u16, u16),
    #[error("Too small output buffer: found {0}, expected {1} bytes")]
    TooSmallOutputBuffer(usize, usize),
    #[error("Output buffer is empty: expected at least {0} bytes")]
    EmptyOutputBuffer(usize),
    #[error("Input stream finished early: {0}")]
    InputReaderError(#[from] std::io::Error),
    #[error("Input incomplete ({0} of at least {1})")]
    Incomplete(usize, usize),
}

pub type Result<T> = std::result::Result<T, self::Error>;
