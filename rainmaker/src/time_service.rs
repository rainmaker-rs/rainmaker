use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use serde_json::Value;

use crate::node::Node;
use crate::param::{Param, ParamProperty, ParamTypes, ParamValue};
use crate::device::{Device, DeviceType};

const SERVICE_NAME: &str = "Time";
const PARAM_NAME_TZ: &str = "TZ";
const PARAM_NAME_TZPOSIX: &str = "TZ-POSIX";

fn create_tz_param(tz: String) -> Param {
    let mut param_properties = HashSet::new();
    param_properties.insert(ParamProperty::Read);
    param_properties.insert(ParamProperty::Write);
    let timezone = Param::new(PARAM_NAME_TZ, ParamValue::String(tz), ParamTypes::Timezone, param_properties);
    timezone
}

fn create_tz_posix_param(tz_posix: String) -> Param {
    let mut param_properties = HashSet::new();
    param_properties.insert(ParamProperty::Read);
    param_properties.insert(ParamProperty::Write);
    let tzposix = Param::new(PARAM_NAME_TZPOSIX, ParamValue::String(tz_posix), ParamTypes::TimezonePOSIX, param_properties);
    tzposix
}

pub(crate)fn time_callback(params: HashMap<String, Value>) {
    log::info!("Received update: {:?}", params);
    // ToDo
}

pub(crate) fn enable_timezone(node: Arc<Mutex<Node>>, tz: String, tz_posix: String) {
    let tz = create_tz_param(tz);
    let tzposix = create_tz_posix_param(tz_posix);
    let mut time = Device::new(SERVICE_NAME, DeviceType::Time);
    time.add_param(tz);
    time.add_param(tzposix);
    time.register_callback(Box::new(time_callback));
    let mut locked_node = node.lock().unwrap();
    locked_node.add_service(time);    
}
