use std::sync::mpsc::{self, Receiver, Sender};

use quick_protobuf::{MessageWrite, Writer};
use serde_json::json;
pub use uuid::Uuid;

mod base;
mod ble;
mod softap;

use crate::{
    error::Error,
    persistent_storage::{Nvs, NvsPartition},
    protocomm::{ProtocommCallbackType, ProtocommSecurity},
    utils::{wrap_in_arc_mutex, WrappedInArcMutex},
    wifi::{WifiApInfo, WifiClientConfig, WifiMgr},
};

pub use base::WiFiProvTransportTrait;

use ble::WiFiProvTransportBle;
pub use ble::WifiProvBleConfig;

pub use softap::WiFiProvSoftApConfig;
use softap::WiFiProvTransportSoftAp;

pub type WiFiProvMgrBle = WifiProvMgr<WiFiProvTransportBle>;
pub type WiFiProvMgrSoftAp = WifiProvMgr<WiFiProvTransportSoftAp>;

const WIFI_NAMESPACE: &str = "net80211";
const WIFI_SSID_KEY: &str = "sta.ssid";
const WIFI_PASS_KEY: &str = "sta.pswd";

// A struct for storing shared callback data between various callback functions
struct ProvisioningSharedData {
    wifi: WrappedInArcMutex<WifiMgr<'static>>,
    scan_results: Option<Vec<WifiApInfo>>,
    nvs_partition: NvsPartition,
    msg_sender: Sender<()>,
}

pub struct WifiProvMgr<T>
where
    T: WiFiProvTransportTrait,
{
    sec_ver: u8,
    pop: Option<String>,
    shared: WrappedInArcMutex<ProvisioningSharedData>,
    transport: T,
    msg_receiver: Receiver<()>,
}

impl WiFiProvMgrSoftAp {
    pub fn new(
        wifi: WrappedInArcMutex<WifiMgr<'static>>,
        config: WiFiProvSoftApConfig,
        nvs_partition: NvsPartition,
        sec: ProtocommSecurity,
    ) -> Result<WiFiProvMgrSoftAp, Error> {
        let (sec_ver, pop) = Self::get_sec_ver_and_pop(&sec);
        let transport = WiFiProvTransportSoftAp::new(config, sec, wifi.clone());
        Self::new_with_transport(wifi, nvs_partition, sec_ver, pop, transport)
    }
}

impl WiFiProvMgrBle {
    pub fn new(
        wifi: WrappedInArcMutex<WifiMgr<'static>>,
        config: WifiProvBleConfig,
        nvs_partition: NvsPartition,
        sec: ProtocommSecurity,
    ) -> Result<WiFiProvMgrBle, Error> {
        let (sec_ver, pop) = Self::get_sec_ver_and_pop(&sec);
        let transport = WiFiProvTransportBle::new(config, sec);
        Self::new_with_transport(wifi, nvs_partition, sec_ver, pop, transport)
    }
}

impl<T: WiFiProvTransportTrait> WifiProvMgr<T> {
    pub fn wait_for_provisioning(&self) {
        self.msg_receiver.recv().unwrap()
    }

    pub fn add_endpoint(&mut self, ep_name: &str, cb: ProtocommCallbackType) {
        self.transport.add_endpoint(ep_name, cb);
    }
    pub fn start(&mut self) -> Result<(), Error> {
        self.register_protocomm_endpoints();

        self.transport.start()?;
        let data = self.shared.lock().unwrap();
        let mut wifi = data.wifi.lock().unwrap();
        wifi.set_client_config(WifiClientConfig::default()).unwrap();

        // Start WiFi
        wifi.start().unwrap();

        Ok(())
    }

    pub fn is_provisioned(&self) -> Option<(String, String)> {
        let partition = self.shared.lock().unwrap().nvs_partition.clone();
        let nvs = Nvs::new(partition, WIFI_NAMESPACE).expect("Unable to open NVS partition");
        let mut buff = [0; 64];
        let ssid = nvs
            .get_string(WIFI_SSID_KEY, &mut buff)
            .expect("Unable to get SSID");
        let password = nvs
            .get_string(WIFI_PASS_KEY, &mut buff)
            .expect("Unable to get Password");

        if let (Some(ssid), Some(password)) = (ssid, password) {
            return Some((ssid, password));
        }
        None
    }

    fn new_with_transport(
        wifi: WrappedInArcMutex<WifiMgr<'static>>,
        nvs_partition: NvsPartition,
        sec_ver: u8,
        pop: Option<String>,
        transport: T,
    ) -> Result<WifiProvMgr<T>, Error> {
        let (sender, receiver) = mpsc::channel::<()>();

        let shared = wrap_in_arc_mutex(ProvisioningSharedData {
            nvs_partition,
            wifi,
            scan_results: None,
            msg_sender: sender,
        });

        Ok(WifiProvMgr {
            sec_ver,
            pop,
            shared,
            transport,
            msg_receiver: receiver,
        })
    }

    fn get_sec_ver_and_pop(sec: &ProtocommSecurity) -> (u8, Option<String>) {
        let sec_ver;
        let pop;

        match &sec {
            ProtocommSecurity::Sec0(_sec0) => {
                sec_ver = 0;
                pop = None;
            }
            ProtocommSecurity::Sec1(sec1) => {
                sec_ver = 1;
                pop = sec1.pop.clone();
            }
        }

        (sec_ver, pop)
    }

    fn get_version_info(&self) -> String {
        let mut cap = vec!["wifi_scan"];

        if self.pop.is_none() {
            cap.push("no_pop");
        }

        if self.sec_ver == 0 {
            cap.push("no_sec");
        }

        json! ({
            "prov": {
                "ver": "v1.1",
                "sec_ver": self.sec_ver,
                "cap": cap
            }
        })
        .to_string()
    }

    fn register_protocomm_endpoints(&mut self) {
        let version_info = self.get_version_info();
        let shared_1 = self.shared.clone();
        let shared_2 = self.shared.clone();

        self.transport.set_version_info("proto-ver", version_info);
        self.transport.set_security_ep("prov-session");
        self.add_endpoint(
            "prov-scan",
            Box::new(move |ep, data| ep_prov_scan::prov_scan_callbak(ep, data, shared_1.clone())),
        );
        self.add_endpoint(
            "prov-config",
            Box::new(move |ep, data| {
                ep_prov_config::prov_config_callback(ep, data, shared_2.clone())
            }),
        );
    }
}

mod ep_prov_scan {
    use super::*;

    use crate::proto::{
        constants::Status,
        wifi_prov::{
            wifi_constants::WifiAuthMode,
            wifi_scan::{mod_WiFiScanPayload::OneOfpayload, *},
        },
    };

    impl From<WifiApInfo> for WiFiScanResult {
        fn from(value: WifiApInfo) -> Self {
            let auth = match value.auth {
                crate::wifi::WifiAuthMode::None => WifiAuthMode::Open,
                crate::wifi::WifiAuthMode::WEP => WifiAuthMode::WEP,
                crate::wifi::WifiAuthMode::WPA => WifiAuthMode::WPA_PSK,
                crate::wifi::WifiAuthMode::WPA2Personal => WifiAuthMode::WPA2_PSK,
                crate::wifi::WifiAuthMode::WPAWPA2Personal => WifiAuthMode::WPA_WPA2_PSK,
                crate::wifi::WifiAuthMode::WPA2Enterprise => WifiAuthMode::WPA2_ENTERPRISE,
                crate::wifi::WifiAuthMode::WPA3Personal => WifiAuthMode::WPA3_PSK,
                crate::wifi::WifiAuthMode::WPA2WPA3Personal => WifiAuthMode::WPA2_WPA3_PSK,
                _ => panic!("Unknown WiFi auth type"),
            };

            Self {
                ssid: value.ssid.into(),
                channel: value.channel as u32,
                bssid: value.bssid,
                rssi: value.signal_strength as i32,
                auth,
            }
        }
    }

    #[inline(always)]
    pub fn prov_scan_callbak(
        _ep: &str,
        inp: &[u8],
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> Vec<u8> {
        let mut out_payload: Vec<u8> = Default::default();
        let mut writer = Writer::new(&mut out_payload);
        let mut resp = WiFiScanPayload::default();

        let inp_data = match WiFiScanPayload::try_from(inp) {
            Ok(payload) => payload,
            Err(_) => {
                resp.status = Status::InvalidProto;
                resp.write_message(&mut writer).unwrap();
                return out_payload;
            }
        };

        let resp_msg;
        let resp_payload = match inp_data.payload {
            OneOfpayload::cmd_scan_start(cmd_scan_start) => {
                resp_msg = WiFiScanMsgType::TypeRespScanStart;
                handle_scan_start(cmd_scan_start, shared)
            }
            OneOfpayload::cmd_scan_status(cmd_scan_status) => {
                resp_msg = WiFiScanMsgType::TypeRespScanStatus;
                handle_scan_status(cmd_scan_status, shared)
            }
            OneOfpayload::cmd_scan_result(cmd_scan_result) => {
                resp_msg = WiFiScanMsgType::TypeRespScanResult;
                handle_scan_result(cmd_scan_result, shared)
            }
            other => {
                log::error!("Invalid payload type {:?}", other);
                return vec![];
            }
        };

        resp.status = Status::Success;
        resp.msg = resp_msg;
        resp.payload = resp_payload;

        if resp.write_message(&mut writer).is_err() {
            log::error!("Failed to write message");
            return vec![];
        };

        out_payload
    }

    fn handle_scan_start(
        _cmd: CmdScanStart,
        _shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        let resp = RespScanStart::default();
        OneOfpayload::resp_scan_start(resp)
    }

    fn handle_scan_status(
        _cmd: CmdScanStatus,
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        let mut resp = RespScanStatus::default();

        let mut data = shared.lock().unwrap();

        let networks = data.wifi.lock().unwrap().scan().unwrap();

        resp.scan_finished = true;
        resp.result_count = networks.len() as u32;
        log::info!("Found {} WiFi network(s)", networks.len());

        data.scan_results = Some(networks);

        OneOfpayload::resp_scan_status(resp)
    }

    fn handle_scan_result(
        cmd: CmdScanResult,
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        log::info!("Sending WiFi scan results");

        let mut resp = RespScanResult::default();

        let mut data = shared.lock().unwrap();
        let networks = data
            .scan_results
            .as_mut()
            .expect("WiFi scan results not found");

        let start_index = cmd.start_index as usize;
        let count = cmd.count as usize;
        let end_index = start_index + count;

        let entries = networks
            .drain(start_index..end_index)
            .map(|x| x.into())
            .collect();

        resp.entries = entries;
        OneOfpayload::resp_scan_result(resp)
    }
}

mod ep_prov_config {
    use mod_RespGetStatus::OneOfstate;
    use mod_WiFiConfigPayload::OneOfpayload;
    use quick_protobuf::{MessageWrite, Writer};

    use super::*;
    use crate::{
        proto::wifi_prov::{
            constants::*,
            wifi_config::*,
            wifi_constants::{WifiConnectFailedReason, WifiConnectedState, WifiStationState},
        },
        wifi::WifiClientConfig,
    };

    use super::ProvisioningSharedData;

    #[inline(always)]
    pub fn prov_config_callback(
        _ep: &str,
        inp: &[u8],
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> Vec<u8> {
        let mut resp = WiFiConfigPayload::default();
        let mut out_vec = Vec::<u8>::new();
        let mut writer = Writer::new(&mut out_vec);

        let inp_payload = WiFiConfigPayload::try_from(inp).unwrap();

        let resp_payload = match inp_payload.payload {
            mod_WiFiConfigPayload::OneOfpayload::cmd_get_status(cmd_get_status) => {
                resp.msg = WiFiConfigMsgType::TypeRespGetStatus;
                handle_get_status(cmd_get_status, shared)
            }
            mod_WiFiConfigPayload::OneOfpayload::cmd_set_config(cmd_set_config) => {
                resp.msg = WiFiConfigMsgType::TypeRespSetConfig;
                handle_set_config(cmd_set_config, shared)
            }
            mod_WiFiConfigPayload::OneOfpayload::cmd_apply_config(cmd_apply_config) => {
                resp.msg = WiFiConfigMsgType::TypeRespApplyConfig;
                handle_apply_config(cmd_apply_config, shared)
            }
            _ => unreachable!(),
        };

        resp.payload = resp_payload;

        if resp.write_message(&mut writer).is_err() {
            log::error!("Failed to write wifi_config response");
            return vec![];
        };

        out_vec
    }

    fn handle_set_config(
        cmd: CmdSetConfig,
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        let mut resp = RespSetConfig::default();

        let ssid = String::from_utf8(cmd.ssid).expect("Failed to decode WiFi SSID");
        let password = String::from_utf8(cmd.passphrase).expect("Failed to decode WiFi passphrase");
        let bssid = cmd.bssid;
        let channel = cmd.channel;

        log::info!("Received SSID={} PASSWORD={}", ssid, password);

        let wifi_config = WifiClientConfig {
            ssid,
            password,
            bssid,
            channel: channel as u8,
            ..Default::default()
        };

        // SSID and Password are saved after connection so as to deal with incorrect password

        let data = shared.lock().unwrap();
        data.wifi
            .lock()
            .unwrap()
            .set_client_config(wifi_config)
            .unwrap();

        resp.status = Status::Success;

        OneOfpayload::resp_set_config(resp)
    }

    fn handle_apply_config(
        _cmd: CmdApplyConfig,
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        log::info!("Connecting to WiFi");
        let mut resp = RespApplyConfig::default();

        let data = shared.lock().unwrap();
        let mut wifi = data.wifi.lock().unwrap();

        if wifi.connect().is_err() {
            log::error!("Failed connecting to provided WiFi network");
        } else {
            let (client_config, _) = wifi.get_wifi_config();
            if let Some(config) = client_config {
                let ssid = config.ssid;
                let password = config.password;
                let nvs_partition = data.nvs_partition.clone();
                let nvs = Nvs::new(nvs_partition, WIFI_NAMESPACE);
                match nvs {
                    Err(_) => log::error!("Failed to open nvs for saving WiFi credentials"),
                    Ok(mut nvs) => {
                        nvs.set_str(WIFI_SSID_KEY, &ssid)
                            .expect("Failed to save SSID");
                        nvs.set_str(WIFI_PASS_KEY, &password)
                            .expect("Failed to save Password");
                    }
                }
            }
        }
        resp.status = Status::Success;

        OneOfpayload::resp_apply_config(resp)
    }

    fn handle_get_status(
        _cmd: CmdGetStatus,
        shared: WrappedInArcMutex<ProvisioningSharedData>,
    ) -> OneOfpayload {
        let mut resp = RespGetStatus::default();

        let data = shared.lock().unwrap();

        // TODO: send actual data
        let wifi = data.wifi.lock().unwrap();
        let ip_addr = wifi.get_ip_addr();

        resp.status = Status::Success;
        if wifi.is_connected() {
            resp.sta_state = WifiStationState::Connected;
            resp.state = OneOfstate::connected(WifiConnectedState {
                ip4_addr: ip_addr.to_string(),
                ..Default::default()
            });
        } else {
            resp.sta_state = WifiStationState::ConnectionFailed;
            resp.state = OneOfstate::fail_reason(WifiConnectFailedReason::AuthError);
        }

        data.msg_sender.send(()).unwrap();

        OneOfpayload::resp_get_status(resp)
    }
}
