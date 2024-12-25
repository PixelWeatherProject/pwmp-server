use std::{io, num::TryFromIntError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Failed to parse a `Message`.
    #[error("Failed to parse message")]
    MessageParse,

    /// Expected a message of type `Request`, got `Response` instead.
    #[error("Expected message of variant `Request`, got `Response` instead")]
    NotRequest,

    /// Expected the first message to be of type `Hello`.
    #[error("Expected a `Hello` request")]
    NotHello,

    /// Request was malformed or cannot be processed.
    #[error("Malformed or unprocessable request")]
    BadRequest,

    /// Connection closed unexpectedly.
    #[error("Connection closed unexpectedly")]
    Quit,

    /// Database error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database migration error.
    #[error("Database migration error: {0}")]
    DatabaseMigration(#[from] sqlx::migrate::MigrateError),

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
}
