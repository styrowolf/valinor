use thiserror::Error;
use zeromq::ZmqError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ZeroMQ error: {0:?}")]
    ZeroMq(#[from] ZmqError),
    #[error("The upstream has indicated that it is shutting down and no more work will be sent")]
    UpstreamShuttingDown,
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
}
