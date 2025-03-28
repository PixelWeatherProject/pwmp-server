use std::{io, num::TryFromIntError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to parse a `Message`.
    #[error("Failed to parse message")]
    MessageParse,

    /// Expected a message of type `Request`, got `Response` instead.
    #[error("Expected message of variant `Request`, got `Response` instead")]
    NotRequest,

    /// Expected the first message to be a handshake request.
    #[error("Expected a handshake request")]
    NotHandshake,

    /// Database error.
    #[error("Database error: {0:?}")]
    Database(#[from] DatabaseError),

    /// Failed to set up the logger.
    #[error("Failed to set global logger")]
    LogInit(#[from] log::SetLoggerError),

    /// Integer conversion error.
    #[error("Integer conversion error: {0}")]
    IntConversion(#[from] TryFromIntError),

    /// Generic I/O error
    #[error("I/O: {0}")]
    Io(#[from] io::Error),

    /// Authentication error.
    #[error("Node authentication failed")]
    Auth,

    /// Node stalled for too long.
    #[error("Node stalled for too long")]
    StallTimeExceeded,

    /// Invalid message length.
    #[error("Message length is zero, too large, or generaly invalid")]
    IllegalMessageLength,

    /// The provided buffer was not large enough.
    #[error("The provided buffer is too small")]
    InvalidBuffer,

    /// A message has been received twice.
    #[error("Duplicate message")]
    DuplicateMessage,

    /// Invalid message length.
    #[error("Message is too large to send")]
    MessageTooLarge,
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error(transparent)]
    Sqlite(#[from] r2d2_sqlite::rusqlite::Error),

    #[error(transparent)]
    Postgres(#[from] r2d2_postgres::postgres::Error),
}
