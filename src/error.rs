use std::{io, num::TryFromIntError};

use message_io::network::SendStatus;

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

    /// Network error
    #[error("Network error: {0:?}")]
    Network(SendStatus),

    /// Authentication error.
    #[error("Node authentication failed")]
    Auth,
}

pub trait SendStatusEx {
    fn errorize(self) -> Result<(), Error>;
}

impl SendStatusEx for SendStatus {
    fn errorize(self) -> Result<(), Error> {
        match self {
            Self::Sent => Ok(()),
            _ => Err(Error::Network(self)),
        }
    }
}
