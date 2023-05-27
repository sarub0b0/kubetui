// pub type Result<R> = std::result::Result<R, Box<dyn std::error::Error>>;

pub use anyhow::{anyhow, Result};

use thiserror::Error as TError;

#[derive(Debug, TError)]
pub enum Error {
    #[error(transparent)]
    Kube(#[from] kube::Error),
    #[error("{0}")]
    Raw(String),
    #[error("{0:#?}")]
    VecRaw(Vec<String>),
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Pod(#[from] PodError),
    #[error("NoneParameter: {0}")]
    NoneParameter(&'static str),
}

#[derive(Debug, TError)]
pub enum PodError {
    #[error("ContainerExitCodeNotZero: {0}")]
    ContainerExitCodeNotZero(String, Vec<String>),
}
