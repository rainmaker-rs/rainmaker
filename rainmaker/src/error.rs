use thiserror::Error;

#[derive(Error, Debug)]
pub enum RmakerMqttError {
    #[error("already started")]
    AlreadyInitialized,
    #[error("node credentails not found")]
    NodeCredentialsNotFound,
    #[error("not initialized")]
    NotInitialized,
    #[error("unknown error")]
    OtherError,
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RmakerError {
    #[error("already initialized")]
    AlreadyInitialized,
    #[error("MQTT wrapper error")]
    Mqtt(#[from] RmakerMqttError),
    #[error("factory partition error")]
    Factory(#[from] RmakerFactoryError),
}

#[derive(Error, Debug)]
pub enum RmakerFactoryError {
    #[error("already initialized")]
    AlreadyInitialized,
    #[error("not initialized")]
    NotInitialized,
    #[error("partition not found")]
    PartitionNotFound,
    #[error("value read error")]
    ValueReadError,
}
