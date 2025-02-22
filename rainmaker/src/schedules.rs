
//"Schedule":{"Schedules":[{"name":"s1","id":"URUE","enabled":true,"action":{"Light":{"Brightness":76,"Hue":48,
//"Power":true,"Saturation":100}},"triggers":[{"m":980,"d":127}]}]}

use serde::Serialize;
use serde_json::Value;

use crate::param::{ Param, ParamTypes, ParamProperty, ParamValue };
use crate::device::{ Device, DeviceType };
use crate::node::Node;

use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

use time::{OffsetDateTime, Weekday};

// Default value of Schedule in the app as a Service
const SERVICE_NAME: &str = "Schedule";

// Default value of Schedule in the app as a Array of Parameters
const PARAM_NAME: &str = "Schedules";

// This can be modified based on your need.
const MAX_SCHEDULES: usize = 10;

#[derive(Clone, Debug, Serialize)]
struct Trigger {
    // Minutes from 12 am to trigger : MAX VALUE = 24*60 = 1440
    m: u16,
    // Each bit represents a day of week, 0 represents the trigger is valid once, 0b0000001 represents Monday
    d: u8,
    // Date Day : [1-31], ToDo
    dd: Option<u8>,
    // Date Month: [1-12], ToDo
    mm: Option<u16>,
    // Date Year, ToDo
    yy: Option<u16>,
    // Not Implemented Yet, Repeat.
    r: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
struct Schedule {
    name: String,
    id: String,
    enabled: bool,
    action: HashMap<String, HashMap<String, Value>>,
    triggers: Vec<Trigger>,
}

#[derive(Clone, Debug)]
struct PrivScheduleData {
    schedule_list: Vec<Schedule>,
    total_schedules: usize,
}

impl PrivScheduleData {
    fn new() -> PrivScheduleData {
        Self { 
            schedule_list: vec![], 
            total_schedules: 0,
        }
    }
}

pub(self) static SCHEDULE: OnceLock<RwLock<PrivScheduleData>> = OnceLock::new();

fn get_shedule_data() -> &'static RwLock<PrivScheduleData> {
    SCHEDULE.get_or_init(|| RwLock::new(PrivScheduleData::new()))
}

fn get_current_minutes_and_weekday() -> (u16, u8) {
    let curr_time = time::OffsetDateTime::now_local().unwrap();
    let mins: u16 = ((curr_time.hour() as u16) * 60) + (curr_time.minute() as u16);
    let weekday = curr_time.weekday();
    let weekday_in_u8: u8 = match weekday {
        Weekday::Sunday => 1 << 6,      // 0x01000000
        Weekday::Saturday => 1 << 5,      // 0x00100000
        Weekday::Friday => 1 << 4,     // 0x00010000
        Weekday::Thursday => 1 << 3,   // 0x00001000
        Weekday::Wednesday => 1 << 2,    // 0x00000100
        Weekday::Tuesday => 1 << 1,      // 0x00000010
        Weekday::Monday => 1 << 0,    // 0x00000001
    };
    (mins, weekday_in_u8)
}

fn find_next_trigger(schedules: &Vec<Schedule>) -> Option<(Schedule, u16)> {
    let mut next_schd: Option<(Schedule, u16)> = None;
    let mut closest: Option<u16> = None;
    let (current_mins, current_day) = get_current_minutes_and_weekday(); 
    for schd in schedules {
        if schd.enabled {
            for trig in &schd.triggers {
                let time_diff = trig.m as i16 - current_mins as i16;
                if (trig.d & current_day) != 0 { 
                    if (time_diff>0) & (closest.is_some_and(|tim: u16| tim as i16 > time_diff) | closest.is_none()) {
                        closest = Some(time_diff as u16);
                        next_schd = Some((schd.clone(), closest.unwrap()));
                    }   
                }
            }
            if next_schd.is_none() {
                for trig in &schd.triggers {
                    let time_diff = trig.m as i16 - current_mins as i16 + 1440;                         // 1440 = 24 * 60
                    if ((trig.d & (current_day >> 1)) != 0) | ((trig.d & (current_day << 5)) != 0) {        // Edge case for the next day. ToDo: Need to be looked at again.
                        if (time_diff>0) & (closest.is_some_and(|tim: u16| tim as i16 > time_diff) | closest.is_none()) {
                            closest = Some(time_diff as u16);
                            next_schd = Some((schd.clone(), closest.unwrap()));
                        }   
                    }
                }
            }
        }
    }
    next_schd
}

fn execute_schedule_action(schedule: &Schedule, node: &Arc<Mutex<Node>>) {
    log::info!("Executing action for schedule with id: {:?}", schedule.id);
    let node = node.lock().unwrap();
    for (device, params) in &schedule.action {
        node.exeute_device_callback(device, params.to_owned());        
    }
}

pub(crate) fn wait_for_next_trigger(node: &Arc<Mutex<Node>>) {
    loop {
        let rwlock = get_shedule_data();
        let priv_schedules = rwlock.read().unwrap();
        let schedules = &priv_schedules.schedule_list;
        if let Some((schedule, wait_time)) = find_next_trigger(&schedules) {            
            log::info!(
                "Next trigger for {} in {} minutes",
                schedule.name, wait_time
            );

            thread::sleep(Duration::from_secs((wait_time * 60) as u64));
            if schedule.triggers[0].d == 0 {
                disable_schedule(Some(schedule.id.clone()));
            }
            execute_schedule_action(&schedule, node);
        } else {
            thread::sleep(Duration::from_secs(60));
        }
    }
}

fn start_schedule_thread(node: Arc<Mutex<Node>>) {
    thread::spawn(move || {
        wait_for_next_trigger(&node);
    });
}

pub(crate) fn schedule_callback(params: HashMap<String, Value>) {
    log::info!("Received update: {:?}", params);
    
    for (key, value) in params {
        match key.as_str() {
            PARAM_NAME => {
                if let Value::Array(arr) = value {
                    let scheds: Vec<HashMap<String, Value>> = arr.iter()
                        .filter_map(|v| v.as_object().map(|m| m.iter().map(|(k,v)| (k.clone(), v.clone())).collect()))
                        .collect();

                    for sched in scheds {
                        let mut name: Option<String> = None;
                        let mut id: Option<String> = None;
                        let mut action: Option<HashMap<String, HashMap<String, Value>>> = None;
                        let mut triggers: Option<Vec<Trigger>> = None;
                        let mut operation: Option<String> = None;
                        for (k, v) in sched {
                            match k.as_str() {
                                "name" => name = Some(v.as_str().unwrap().to_string()),
                                "id" => id = Some(v.as_str().unwrap().to_string()),
                                "action" => action = serde_json::from_str(v.as_str().unwrap()).unwrap(),
                                "operation" => operation = Some(v.as_str().unwrap().to_string()),
                                "triggers" => {
                                    triggers = v.as_array().map(|arr| {
                                        arr.iter()
                                        .filter_map(|t| {
                                            if let Some(obj) = t.as_object() {
                                                Some(Trigger {
                                                    m: obj.get("m")?.as_u64()? as u16,
                                                    d: obj.get("d")?.as_u64()? as u8,
                                                    dd: obj.get("dd").and_then(|x| x.as_u64().map(|v| v as u8)),
                                                    mm: obj.get("mm").and_then(|x| x.as_u64().map(|v| v as u16)),
                                                    yy: obj.get("yy").and_then(|x| x.as_u64().map(|v| v as u16)),
                                                    r: obj.get("r").and_then(|x| x.as_bool()),
                                                })
                                            } else {
                                                None
                                            }
                                        })
                                        .collect()
                                    });
                                },
                                _ => log::debug!("Received Unknown Value in Schedules"),
                            }
                        }
                        match operation.unwrap().as_str() {
                            "add" => add_schedule(id, name, action, triggers),
                            "edit" => edit_schedule(id, name, action, triggers),
                            "remove" => remove_schedule(id),
                            "enable" => enable_schedule(id),
                            "disable" => disable_schedule(id),
                            _ => log::debug!("Unknown Action received in Schedules"),
                        }
                    }
                } else {
                    log::warn!("Expected an array of strings for '{}', but got: {:?}", key, value);
                }
            },
            _ => log::debug!("Invalid parameter received in schedules: {}", key),
        }
    }
}

pub(crate) fn enable_schedules(node: Arc<Mutex<Node>>) {
    let mut param_properties = HashSet::new();
    param_properties.insert(ParamProperty::Read);
    param_properties.insert(ParamProperty::Write);
    let schedules = Param::new(PARAM_NAME, ParamValue::Array(vec![]), ParamTypes::Schedules, param_properties);
    let mut schedule =  Device::new(SERVICE_NAME, DeviceType::Schedule);
    schedule.add_param(schedules);
    schedule.register_callback(Box::new(schedule_callback));
    let mut locked_node = node.lock().unwrap();
    locked_node.add_service(schedule);
    start_schedule_thread(node.clone());
}


// {"Schedule":{"Schedules":[{"action":{"Light":{"Brightness":76,"Hue":48,"Power":true,"Saturation":100}},
// "id":"URUE","name":"s1","operation":"add","triggers":[{"d":127,"m":915}]}]}}
fn add_schedule(id: Option<String>, name: Option<String>, action: Option<HashMap<String, HashMap<String, Value>>>, triggers: Option<Vec<Trigger>>) {
    let id = id.unwrap();
    let name = name.unwrap();
    let action = action.unwrap();
    let triggers = triggers.unwrap();

    let mut locked_schd = get_shedule_data().write().unwrap();
    if locked_schd.total_schedules < MAX_SCHEDULES {
        let schd: Schedule = Schedule {
            name: name.to_string(),
            id: id.to_string(),
            enabled: true,
            action,
            triggers
        };
        for schedule in &locked_schd.schedule_list {
            if schedule.id == id {
                log::error!("Schedule with id '{}' already exists.", id);
                return;
            }
        }
        locked_schd.schedule_list.push(schd);
        locked_schd.total_schedules += 1;
    } else {
        log::error!("Max number of Schedules reached. Failed to create a Schedule.");
    }
}


// {"Schedule":{"Schedules":[{"action":{"Light":{"Brightness":76,"Hue":48,"Power":true,"Saturation":100}},
// "id":"URUE","name":"s1","operation":"edit","triggers":[{"d":127,"m":917}]}]}}
fn edit_schedule(id: Option<String>, name: Option<String>, action: Option<HashMap<String, HashMap<String, Value>>>, triggers: Option<Vec<Trigger>>) {
    let id = id.unwrap();
    let mut locked_schd = get_shedule_data().write().unwrap();
    let mut found = false;
    for mut schedule in &mut locked_schd.schedule_list {
        if schedule.id == id {
            found = true;
            if name.is_some() {
                schedule.name = name.unwrap().to_string();
            }
            if action.is_some() {
                schedule.action = action.unwrap();
            }
            if triggers.is_some() {
                schedule.triggers = triggers.unwrap();
            }
            break;
        }
    }
    if !found {
        log::error!("Could not find schedule with id {id} to edit");
    }
}

fn remove_schedule(id: Option<String>) {
    let id = id.unwrap();
    let mut locked_schd = get_shedule_data().write().unwrap();
    let mut idx = 0;
    for schedule in &mut locked_schd.schedule_list {
        if schedule.id == id {
            locked_schd.schedule_list.swap_remove(idx);
            break;
        }
        idx += 1;
    }
}


// {"Schedule":{"Schedules":[{"id":"URUE","operation":"enable"}]}}
fn enable_schedule(id: Option<String>) {
    let id = id.unwrap();
    let mut locked_schd = get_shedule_data().write().unwrap();
    for schedule in &mut locked_schd.schedule_list {
        if schedule.id == id {
            schedule.enabled = true;
        }
    }
}

// {"Schedule":{"Schedules":[{"id":"URUE","operation":"disable"}]}}
fn disable_schedule(id: Option<String>) {
    let id = id.unwrap();
    let mut locked_schd = get_shedule_data().write().unwrap();
    for schedule in &mut locked_schd.schedule_list {
        if schedule.id == id {
            schedule.enabled = false;
        }
    }
}