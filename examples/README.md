# Examples 

## 1. Switch
- [Code](src/bin/switch/switch.rs)
- How to run 
```bash
cargo run --target <target> --bin switch
```
- What to expect   
This is a basic example demonstrating building applications intended for running on ESP32 as well as on Linux systems. Provides a `Switch` device having an On/Off toggle representing an actual switch. Toggling power in the phone application logs corresponding change in console.

## 2. LED
- [Code](src/bin/led/led.rs)
- How to run 
```bash
cargo run --target <target> --bin led
```
- What to expect   
This is an example for building cross platform applications with changes only required in the driver. Provides a `LED` device having an On/Off toggle, and slider controls for brightness, saturation and hue.    
It controls the onboard addressable LED connected on various ESP devkits. By default it controls the LED connected to GPIO8, you may need to tweak the pin in the `main` function(in the call `WS2812RMT::new`) if your devkit has the LED on some other pin isn't the same.   
On linux it simply logs the changes in values.

### General Instructions about examples
- While running any example for the first, you will need to perform the provisioning process.   
It will associate the node control to the RainMaker account on the phone app as well as provide the the credentials for WiFi connectivity.  

- For provisioning, you'll need to select the add device option and then select the device manually using BLE scheme in the phone app(device name is `PROV_SERVICE`). Follow onscreen prompts thereafter.

- Bluetooth needs to be powered on on Linux before executing exe

- Reseting WiFi Provisioning  
If you need to re-provision the node for any reasons, here's how you can do it.  
```bash
    ESP32:
$ espflash erase-parts nvs --partition-table partitions.csv
```
```bash
    Linux: 
Delete all the files in ~/.config/rmaker/nvs
```

**Note**: For linux platform, you will need to connect WiFi externally(before running the example) and the provisioning workflow is only used for user-node association. This is because the WiFi stubs are not implemented for linux yet.
