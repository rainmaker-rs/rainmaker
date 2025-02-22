//! A module for Parameters. One or more parameters can be assigned with multiple devices.
//!
//! A single instance of parameter can be assigned to multiple devices.
//!
//! Initialization for standard parameters(Power, Brightness, Hue, Saturation) can be done using specified standard methods.

use serde::Serialize;
use serde_json::{Number, Value};
use std::collections::HashSet;

#[derive(Debug, Serialize)]
pub struct Param {
    name: String,
    param_type: ParamTypes,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui_type: Option<ParamUi>,
    properties: HashSet<ParamProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bounds: Option<ParamBounds>,
    #[serde(rename = "data_type")]
    value: ParamValue,
}

/// Set of access mode parameter.
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamProperty {
    Read,
    Write,
}

/// Set of the type of parameter value.
#[derive(Debug, Clone)]
pub enum ParamValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
    Array(Vec<ParamValue>),
}

/// Set of the parameter type.
///
/// ESP RainMaker provides a set of standard parameters. These are provided with a UI and have special handling in clients like Alexa/Google Home.
///
/// Refer [device list](https://rainmaker.espressif.com/docs/standard-types).
#[derive(Debug, Serialize)]
pub enum ParamTypes {
    #[serde(rename = "esp.param.name")]
    Name,
    #[serde(rename = "esp.param.power")]
    Power,
    #[serde(rename = "esp.param.brightness")]
    Brightness,
    #[serde(rename = "esp.param.cct")]
    CCT,
    #[serde(rename = "esp.param.hue")]
    Hue,
    #[serde(rename = "esp.param.saturation")]
    Saturation,
    #[serde(rename = "esp.param.intensity")]
    Intensity,
    #[serde(rename = "esp.param.speed")]
    Speed,
    #[serde(rename = "esp.param.direction")]
    Direction,
    #[serde(rename = "esp.param.temperature")]
    AmbientTemperature,
    #[serde(rename = "esp.param.setpoint-temperature")]
    TargetTemperature,
    #[serde(rename = "esp.param.humidity")]
    AmbientHumidity,
    #[serde(rename = "esp.param.ota_url")]
    OTAURL,
    #[serde(rename = "esp.param.ota_status")]
    OTAStatus,
    #[serde(rename = "esp.param.ota_info")]
    OTAInfo,
    #[serde(rename = "esp.param.tz")]
    Timezone,
    #[serde(rename = "esp.param.tz_posix")]
    TimezonePOSIX,
    #[serde(rename = "esp.param.schedules")]
    Schedules,
    #[serde(rename = "esp.param.reboot")]
    Reboot,
    #[serde(rename = "esp.param.factory-reset")]
    FactoryReset,
    #[serde(rename = "esp.param.wifi-reset")]
    WiFiReset,
    #[serde(rename = "esp.param.toggle")]
    ToggleController,
    #[serde(rename = "esp.param.range")]
    RangeController,
    #[serde(rename = "esp.param.mode")]
    ModeController,
    #[serde(rename = "esp.param.lockstate")]
    LockState,
    #[serde(rename = "esp.param.blinds-position")]
    BlindsPosition,
    #[serde(rename = "esp.param.garage-position")]
    GaragePosition,
    #[serde(rename = "esp.param.light-mode")]
    LightMode,
    #[serde(rename = "esp.paran.ac-mode")]
    ACMode,
    #[serde(rename = "esp.param.media-activity-state")]
    MediaState,
    #[serde(rename = "esp.param.media-activity-control")]
    MediaControl,
    #[serde(rename = "esp.param.volume")]
    Volume,
    #[serde(rename = "esp.param.mute")]
    Mute,
    #[serde(rename = "esp.param.app-selector")]
    AppSelector,
    #[serde(rename = "esp.param.input-selector")]
    InputSelector,
    #[serde(rename = "esp.param.contact-detection-state")]
    ContactDetectionState,
    #[serde(rename = "esp.param.motion-detection-state")]
    MotionDetectionState,
    #[serde(rename = "esp.param.arm-state")]
    ArmState,
    #[serde(rename = "esp.param.fire-alarm")]
    FireAlarmState,
    #[serde(rename = "esp.param.water-alarm")]
    WaterAlarmState,
    #[serde(rename = "esp.param.carbon-monoxide-alarm")]
    CODetectionState,
    #[serde(rename = "esp.param.burglary-alarm")]
    BurglaryAlarmState,
}

/// Set of standard UI elements.
#[derive(Debug, Serialize)]
pub enum ParamUi {
    #[serde(rename = "esp.ui.text")]
    Text,
    #[serde(rename = "esp.ui.toggle")]
    ToggleSwitch,
    #[serde(rename = "esp.ui.slider")]
    Slider,
    #[serde(rename = "esp.ui.hue-slider")]
    HueSlider,
    #[serde(rename = "esp.ui.hue-circle")]
    HueCircle,
    #[serde(rename = "esp.ui.push-btn-big")]
    PushButton,
    #[serde(rename = "esp.ui.dropdown")]
    Dropdown,
    /// Android Only
    #[serde(rename = "esp.ui.trigger")]
    Trigger,
    /// Android Only
    #[serde(rename = "esp.ui.hidden")]
    Hidden,
}

#[derive(Debug, Serialize)]
struct ParamBounds {
    min: i32,
    max: i32,
    step: i32,
}

impl Param {
    /// Creates a new instance of parameter. Check [ParamValue], [ParamTypes], [ParamProperty], [ParamUi] to pass valid arguments.
    pub fn new(
        name: &str,
        initial_state: ParamValue,
        param_type: ParamTypes,
        properties: HashSet<ParamProperty>,
    ) -> Param {
        Param {
            name: name.to_owned(),
            value: initial_state,
            param_type,
            properties,
            ui_type: None,
            bounds: None,
        }
    }

    /// Returns name of the parameter.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns current state of parameter.
    pub fn value(&self) -> &ParamValue {
        &self.value
    }

    /// Assigns minimum and maximum value to a parameter.
    pub fn add_bounds(&mut self, min: i32, max: i32, step: i32) {
        self.bounds = Some(ParamBounds { min, max, step })
    }

    pub fn add_ui(&mut self, ui: ParamUi) {
        self.ui_type = Some(ui)
    }

    /// Standard function to add Power parameter.
    pub fn new_power(name: &str, initial_value: bool) -> Self {
        let mut param_properties = HashSet::new();
        param_properties.insert(ParamProperty::Read);
        param_properties.insert(ParamProperty::Write);

        let mut param = Self::new(
            name,
            ParamValue::Bool(initial_value),
            ParamTypes::Power,
            param_properties,
        );

        param.add_ui(ParamUi::ToggleSwitch);
        param
    }

    /// Standard function to add Brightness parameter.
    pub fn new_brightness(name: &str, initial_value: u32) -> Self {
        let mut param_properties = HashSet::new();
        param_properties.insert(ParamProperty::Read);
        param_properties.insert(ParamProperty::Write);

        let mut param = Self::new(
            name,
            ParamValue::Integer(initial_value as i64),
            ParamTypes::Brightness,
            param_properties,
        );
        param.add_bounds(0, 100, 1);
        param.add_ui(ParamUi::Slider);

        param
    }

    /// Standard function to add Hue parameter.
    pub fn new_hue(name: &str, initial_value: u32) -> Self {
        let mut param_properties = HashSet::new();
        param_properties.insert(ParamProperty::Read);
        param_properties.insert(ParamProperty::Write);

        let mut param = Self::new(
            name,
            ParamValue::Integer(initial_value as i64),
            ParamTypes::Hue,
            param_properties,
        );
        param.add_bounds(0, 360, 1);
        param.add_ui(ParamUi::HueSlider);

        param
    }

    /// Standard function to add Hue parameter.
    pub fn new_satuation(name: &str, initial_value: u32) -> Self {
        let mut param_properties = HashSet::new();
        param_properties.insert(ParamProperty::Read);
        param_properties.insert(ParamProperty::Write);

        let mut param = Self::new(
            name,
            ParamValue::Integer(initial_value as i64),
            ParamTypes::Saturation,
            param_properties,
        );

        param.add_bounds(0, 100, 1);
        param.add_ui(ParamUi::Slider);

        param
    }
}

impl Serialize for ParamValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            ParamValue::String(_) => "string",
            ParamValue::Bool(_) => "bool",
            ParamValue::Integer(_) => "int",
            ParamValue::Float(_) => "float",
            ParamValue::Array(_) => "array",
        })
    }
}

impl From<ParamValue> for Value {
    fn from(value: ParamValue) -> Self {
        match value {
            ParamValue::String(v) => Self::String(v),
            ParamValue::Bool(v) => Self::Bool(v),
            ParamValue::Integer(v) => Self::Number(Number::from(v)),
            ParamValue::Float(v) => Self::Number(Number::from_f64(v).unwrap()),
            ParamValue::Array(v) => Self::Array(v.into_iter().map(|item| item.into()).collect()),
        }
    }
}
