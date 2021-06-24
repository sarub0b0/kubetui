pub type Result<T, E = Error> = std::result::Result<T, E>;

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
}
