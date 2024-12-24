use thiserror::Error;

#[derive(Error, Debug)]
pub enum RmakerMqttError{
    #[error("already started")]
    AlreadyInitialized,
    #[error("node credentails not found")]
    NodeCredentialsNotFound,
    #[error("not initialized")]
    NotInitialized,
    #[error("unknown error")]
    OtherError
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RmakerError{
    #[error("already initialized")]
    AlreadyInitialized,
    #[error("MQTT wrapper error")]
    Mqtt(#[from] RmakerMqttError)
}