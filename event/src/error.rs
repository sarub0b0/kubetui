// pub type Result<R> = std::result::Result<R, Box<dyn std::error::Error>>;

pub use anyhow::anyhow;
pub use anyhow::Result;
use thiserror::Error as TError;

#[cfg(any(feature = "mock", feature = "mock-failed"))]
#[derive(Debug, TError)]
pub enum Error {
    #[error("MockError: {0}")]
    Mock(&'static str),
    #[error("kubeError: {0}")]
    Kube(#[from] kube::Error),
    #[error("{0}")]
    Raw(String),
    #[error(transparent)]
    IO(#[from] std::io::Error),
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
