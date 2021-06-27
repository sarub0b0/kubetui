pub type Result<R, E = Error> = std::result::Result<R, E>;

use crossbeam::channel::{RecvError, SendError};
use std::fmt::{Debug, Display, Formatter};
use thiserror::Error as TError;

#[cfg(any(feature = "mock", feature = "mock-failed"))]
#[derive(Debug, TError)]
pub enum Error {
    #[error("MockError: {0}")]
    Mock(&'static str),
    #[error("kubeError: {0}")]
    Kube(#[from] kube::Error),
}

#[cfg(not(any(feature = "mock", feature = "mock-failed")))]
#[derive(Debug, TError)]
pub enum Error {
    #[error(transparent)]
    Kube(#[from] kube::Error),
    #[error("{0}")]
    Raw(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}

#[derive(Debug, TError)]
pub enum CrossbeamError<T: Debug> {
    Recv(RecvError),
    Send(SendError<T>),
}

impl<T: Debug> Display for CrossbeamError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<T: Debug> From<SendError<T>> for CrossbeamError<T> {
    fn from(e: SendError<T>) -> Self {
        Self::Send(e)
    }
}

impl<T: Debug> From<RecvError> for CrossbeamError<T> {
    fn from(e: RecvError) -> Self {
        Self::Recv(e)
    }
}
