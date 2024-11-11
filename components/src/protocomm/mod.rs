mod security;
mod transports;

use std::sync::Arc;
use transports::{TransportGatt, TransportHttpd};
use uuid::Uuid;

pub use self::security::ProtocommSecurity;
use self::security::SecurityTrait;
use crate::http;

#[allow(private_interfaces)]
pub type ProtocommHttpd = Protocomm<TransportHttpd>;
#[allow(private_interfaces)]
pub type ProtocommGatt = Protocomm<TransportGatt>;

#[derive(Default)]
pub struct ProtocommGattConfig {
    pub service_uuid: Uuid,
}

pub type ProtocommHttpdConfig = http::HttpConfiguration;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointType {
    Version,
    Security,
    #[default]
    Other,
}

pub type ProtocommCallbackType = Box<dyn Fn(&str, &[u8]) -> Vec<u8> + Send + Sync + 'static>;

pub struct Protocomm<T> {
    transport: T,
    sec: Arc<ProtocommSecurity>,
}

impl Protocomm<TransportGatt> {
    pub fn new(gatt_config: ProtocommGattConfig, security: ProtocommSecurity) -> Self {
        let transport_ble = transports::TransportGatt::new(gatt_config.service_uuid);

        Self {
            transport: transport_ble,
            sec: Arc::new(security),
        }
    }

    pub fn set_version_info(&mut self, uuid: Uuid, ep_name: &str, version_info: String) {
        self.transport.add_endpoint(
            uuid,
            ep_name,
            Box::new(move |_ep, _data| version_info.as_bytes().to_vec()),
            EndpointType::Version,
            self.sec.clone(),
        );
    }

    pub fn set_security_endpoint(&mut self, uuid: Uuid, ep_name: &str) {
        self.transport.add_endpoint(
            uuid,
            ep_name,
            Box::new(|_, _| Vec::default()),
            EndpointType::Security,
            self.sec.clone(),
        );
    }

    pub fn register_endpoint(
        &mut self,
        uuid: Uuid,
        ep_name: &str,
        callback: ProtocommCallbackType,
    ) {
        log::debug!("Registering endpoint: {}", ep_name);
        self.transport.add_endpoint(
            uuid,
            ep_name,
            callback,
            EndpointType::Other,
            self.sec.clone(),
        );
        log::debug!("Registered endpoint: {}", ep_name);
    }

    pub fn start(&mut self) {
        self.transport.start();
    }
}

impl Protocomm<TransportHttpd> {
    pub fn new(config: ProtocommHttpdConfig, security: ProtocommSecurity) -> Self {
        let transport = TransportHttpd::new(config);
        Self {
            transport,
            sec: Arc::new(security),
        }
    }

    pub fn set_version_info(&mut self, ep_name: &str, version_info: String) {
        self.register_endpoint_internal(
            ep_name,
            Box::new(move |_ep, _data| version_info.as_bytes().to_vec()),
            EndpointType::Version,
        );
    }

    pub fn set_security_endpoint(&mut self, ep_name: &str) {
        self.register_endpoint_internal(ep_name, Box::new(|_, _| vec![]), EndpointType::Security);
    }

    pub fn register_endpoint(&mut self, ep_name: &str, callback: ProtocommCallbackType) {
        self.register_endpoint_internal(ep_name, callback, EndpointType::Other);
    }

    fn register_endpoint_internal(
        &mut self,
        ep_name: &str,
        cb: ProtocommCallbackType,
        ep_type: EndpointType,
    ) {
        let sec = self.sec.clone();
        self.transport.add_endpoint(ep_name, cb, ep_type, sec);
    }
}

pub(crate) fn protocomm_req_handler(
    ep: &str,
    data: &[u8],
    cb: &ProtocommCallbackType,
    ep_type: &EndpointType,
    sec: &Arc<ProtocommSecurity>,
) -> Vec<u8> {
    match ep_type {
        EndpointType::Version => cb(ep, data),
        EndpointType::Security => sec.security_handler(ep, data.to_vec()),
        EndpointType::Other => {
            // for decrypting
            let mut data = data.to_vec();

            sec.decrypt(&mut data);
            let mut res = cb(ep, &data);
            sec.encrypt(&mut res);

            res
        }
    }
}
