//! Node module of rainmaker-rs.
//! A node in RainMaker an entity controls one/more devices.
//! Devices can be created and added to the [Node] instance.
//! When there is any change wrt any device(like param update), it's appropriate methods are called
//! by [Node] instance
//!
//! Methods related node are implemented for struct [Node].

/*
Node Id (node_id, String)
Config Version (config_version, String)
Information (info, Object)
    Name (name, String)
    FW Version (fw_version, String)
    Type (type, String)
Node Attributes (attributes, Array of Objects)
    Name (name, String)
    Value (value, String)
Devices (devices, Array of objects)
    Name (name, String)
    Type (type, String)
    Primary (primary, String)
    Device Attributes (attributes, Array of Objects)
        Name (name, String)
        Value (value, String)
    Params (params, Array of objects)
        Name (name, String)
        Data Type (data_type, String)
        Type (type, String)
        Properties (properties, Array of Strings)
        UI Type (ui_type, String)
        Bounds (bounds, Object)
            Minimum (min, Number)
            Maximum (max, Number)
            Step (step, Number)
*/

use std::{collections::HashMap, fmt::Debug};

use serde::Serialize;
use serde_json::Value;

use crate::device::Device;
#[allow(unused)]
use crate::Rainmaker;

#[derive(Debug, Clone, Serialize)]
pub struct Info {
    pub name: String,
    pub fw_version: String,
}

#[derive(Debug, Serialize)]
pub struct Node {
    node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    info: Option<Info>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    attributes: HashMap<String, String>,
    devices: Vec<Device>,
    services: Vec<Device>,
}

impl Node {
    /// An instance of node can be created using `new` method of the module. Node ID should be passed as an argument for the same. Node ID can be obtained from the method [`get_node_id`].
    /// ```rust
    /// let rmaker = Rainmaker::init()?;
    /// let node_id = rmaker.get_node_id();
    /// let mut node = Node::new(node_id);
    /// ```
    ///
    /// [`get_node_id`]: crate::Rainmaker::get_node_id  
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            info: None,
            attributes: HashMap::new(),
            devices: Vec::new(),
            services: Vec::new(),
        }
    }

    /// Node information [Info] (Name, FW Version) is set using this function.
    /// ```rust
    /// node.set_info(Info{
    ///     name: "Example Node".to_string(),
    ///     fw_version: "v1.0".to_string()
    /// });
    /// ```
    pub fn set_info(&mut self, info: Info) {
        self.info = Some(info);
    }

    /// Used to define attributes of node.
    pub fn set_attribute(&mut self, name: String, value: String) {
        self.attributes
            .insert(name, value)
            .expect("Failed to set atttribute");
    }

    /// Multiple devices can be associated with the node by using this method. Instance of device should be passed as an argument.
    ///
    /// Ensure that instance of [device] is created properly and callback is set appropriately in order to report updated parameter values.
    /// ```rust
    /// node.add_device(device);
    /// ```
    ///
    /// [device]: crate::device
    pub fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }

    pub(crate) fn add_service(&mut self, service: Device) {
        self.services.push(service);
    }

    pub(crate) fn get_param_values(&self) -> HashMap<&str, HashMap<&str, Value>> {
        let mut params = HashMap::<&str, HashMap<&str, Value>>::new();
        for dev in &self.devices {
            let mut curr_params = HashMap::<&str, Value>::new();
            for p in dev.params() {
                curr_params.insert(p.name(), p.value().clone().into());
            }
            params.insert(dev.name(), curr_params);
        }

        params
    }

    pub(crate) fn exeute_device_callback(&self, device_name: &str, params: HashMap<String, Value>) {
        for device in self.devices.iter() {
            if device.name() == device_name {
                device.execute_callback(params);
                return;
            }
        }
        for service in self.services.iter() {
            if service.name() == device_name {
                service.execute_callback(params);
                return;
            }
        }
    }
}
