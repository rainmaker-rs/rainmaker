use anyhow::Result;
use examples::{connect_wifi, initializse_logger};
use rainmaker::components::persistent_storage::NvsPartition;
use rainmaker::components::wifi::WifiMgr;
use rainmaker::factory;
use rainmaker::{
    device::{Device, DeviceType},
    node::Node,
    param::Param,
    Rainmaker,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};

const DEVICE_NAME: &str = "LED";
static LED_VALUES: LazyLock<Mutex<(bool, u32, u32, u32)>> =
    LazyLock::new(|| Mutex::new((true, 0, 100, 20)));

mod esp {
    #![cfg(target_os = "espidf")]

    use std::sync::{Mutex, OnceLock};

    use esp_idf_svc::hal::{gpio::OutputPin, peripheral::Peripheral, rmt::RmtChannel};
    use examples::ws2812::WS2812RMT;
    use rgb::RGB8;

    // Power, Hue, Saturation, Value
    static LED_DRIVER: OnceLock<Mutex<WS2812RMT>> = OnceLock::new();

    fn hsv_to_rgb(h: u16, s: u8, v: u8) -> RGB8 {
        let s = s as f64 / 100.0; // Convert to range 0.0 to 1.0
        let v = v as f64 / 100.0; // Convert to range 0.0 to 1.0
        let c = v * s; // Chroma
        let h_prime = h as f64 / 60.0; // Sector index
        let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
        let m = v - c;

        let (r1, g1, b1) = if h_prime <= 1.0 {
            (c, x, 0.0)
        } else if h_prime <= 2.0 {
            (x, c, 0.0)
        } else if h_prime <= 3.0 {
            (0.0, c, x)
        } else if h_prime <= 4.0 {
            (0.0, x, c)
        } else if h_prime <= 5.0 {
            (x, 0.0, c)
        } else if h_prime <= 6.0 {
            (c, 0.0, x)
        } else {
            (0.0, 0.0, 0.0)
        };

        // Convert back to RGB range [0, 255]
        let r = ((r1 + m) * 255.0).round() as u8;
        let g = ((g1 + m) * 255.0).round() as u8;
        let b = ((b1 + m) * 255.0).round() as u8;

        RGB8::new(r, g, b)
    }

    pub fn set_driver(
        pin: impl Peripheral<P = impl OutputPin> + 'static,
        channel: impl Peripheral<P = impl RmtChannel> + 'static,
    ) {
        let driver = WS2812RMT::new(pin, channel).expect("Failed to initialize driver for WS2812");
        let _ = LED_DRIVER.set(Mutex::new(driver));
    }

    pub fn update_led_state(current_values: &(bool, u32, u32, u32)) {
        let color_rgb = match current_values.0 {
            true => hsv_to_rgb(
                current_values.1 as u16,
                current_values.2 as u8,
                current_values.3 as u8,
            ),
            false => RGB8::default(),
        };

        LED_DRIVER
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .set_pixel(color_rgb)
            .unwrap();
    }
}

fn init_led_device() -> Device {
    let mut led_device = Device::new(DEVICE_NAME, DeviceType::Switch);

    let default_values = LED_VALUES.lock().unwrap();

    let power = Param::new_power("Power", default_values.0);
    let hue = Param::new_hue("Hue", default_values.1);
    let saturation = Param::new_satuation("Saturation", default_values.2);
    let brightness = Param::new_brightness("Brightness", default_values.3);

    led_device.add_param(power);
    led_device.add_param(brightness);
    led_device.add_param(saturation);
    led_device.add_param(hue);
    led_device.set_primary_param("Power");

    led_device.register_callback(Box::new(led_cb));
    #[cfg(target_os = "espidf")]
    esp::update_led_state(&default_values);

    led_device
}

fn led_cb(params: HashMap<String, Value>) {
    log::info!("Received update: {:?}", params);

    let mut current_values = LED_VALUES.lock().unwrap();

    for param in params.iter() {
        match param.0.as_str() {
            "Power" => current_values.0 = param.1.as_bool().unwrap(),
            "Hue" => current_values.1 = param.1.as_u64().unwrap() as u32,
            "Saturation" => current_values.2 = param.1.as_u64().unwrap() as u32,
            "Brightness" => current_values.3 = param.1.as_u64().unwrap() as u32,
            _ => {}
        }
    }

    #[cfg(target_os = "espidf")]
    esp::update_led_state(&current_values);
    rainmaker::report_params(DEVICE_NAME, params);
}

pub fn main() -> Result<()> {
    initializse_logger();

    let factory_partition = NvsPartition::new("fctry")?;
    // should be done before Rainmaker::init()
    factory::init(factory_partition)?;

    let rmaker = Rainmaker::init()?;
    let mut node = Node::new(rmaker.get_node_id().to_string());
    node.set_info(rainmaker::node::Info {
        name: "LED Example Node".to_string(),
        fw_version: "v1.0".to_string(),
    });

    #[cfg(target_os = "espidf")]
    {
        let peripherals = esp_idf_svc::hal::peripherals::Peripherals::take()?;
        esp::set_driver(peripherals.pins.gpio8, peripherals.rmt.channel0);
    }

    // Declare it here since we want wifi to be connected after connect_wifi returns
    let wifi_arc_mutex = Arc::new(Mutex::new(WifiMgr::new()?));
    connect_wifi(rmaker, wifi_arc_mutex.clone())?;

    log::info!("WiFi connected successfully");

    let led_device = init_led_device();
    node.add_device(led_device);

    rmaker.register_node(node);
    rmaker.start()?;

    log::info!("Rainmaker agent is started");

    // Inorder to prevent variable dropping from drop
    loop {
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
