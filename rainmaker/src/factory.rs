use rainmaker_components::persistent_storage::{Nvs, NvsPartition};
use std::sync::OnceLock;

use crate::error::RmakerFactoryError;

static PARTITION: OnceLock<NvsPartition> = OnceLock::new();

pub fn init(partition: NvsPartition) -> Result<(), RmakerFactoryError> {
    if PARTITION.get().is_some() {
        return Err(RmakerFactoryError::AlreadyInitialized);
    }

    // Can't fail since PARTITION is not set
    let _ = PARTITION.set(partition);

    Ok(())
}

pub(crate) fn get_node_id(buff: &mut [u8]) -> Result<String, RmakerFactoryError> {
    let bytes = get_bytes_factory("node_id", buff)?;
    // This should not fail if claiming is performed properly
    Ok(String::from_utf8(bytes).unwrap())
}

pub(crate) fn get_client_cert(buff: &mut [u8]) -> Result<Vec<u8>, RmakerFactoryError> {
    get_bytes_factory("client_cert", buff)
}

pub(crate) fn get_client_key(buff: &mut [u8]) -> Result<Vec<u8>, RmakerFactoryError> {
    get_bytes_factory("client_key", buff)
}

pub fn get_client_random(buff: &mut [u8]) -> Result<Vec<u8>, RmakerFactoryError> {
    get_bytes_factory("random", buff)
}

fn get_bytes_factory(nvs_key: &str, buff: &mut [u8]) -> Result<Vec<u8>, RmakerFactoryError>{
    let factory_partition = match PARTITION.get() {
        Some(partition) => partition,
        None => return Err(RmakerFactoryError::NotInitialized),
    };
    let nvs = match Nvs::new(factory_partition.clone(), "rmaker_creds"){
        Ok(nvs) => nvs,
        Err(_) => return Err(RmakerFactoryError::PartitionNotFound),
    };
    let bytes = match nvs.get_bytes(nvs_key, buff){
        Ok(Some(bytes)) => bytes,
        _ => return Err(RmakerFactoryError::ValueReadError),
    };

    Ok(bytes)
}