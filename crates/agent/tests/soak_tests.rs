//! Soak lifecycle simulation tests (Milestone 6: M6.2).
//!
//! Verifies stability across repeated sleep/wake cycles, device arrival/removal,
//! USB <-> Bluetooth switching, and concurrent atomic hot-swapping.

use std::thread;

use magicbridge_agent::ipc::AtomicMappingTable;
use magicbridge_agent::logger;
use magicbridge_agent::state::DaemonState;
use magicbridge_core::{Config, NamedProfile};

#[test]
fn test_soak_lifecycle_sleep_resume() {
    let config = Config::default();
    let mut state = DaemonState::new(config, None);
    state.is_connected = true;
    state.transport_name = Some("Bluetooth".to_string());
    state.battery_percent = Some(95);

    // Simulate 500 sleep/resume cycles
    for i in 0..500 {
        // Suspend event
        let was_connected = state.is_connected;
        assert!(was_connected);

        // Resume event
        let devices = vec![(
            r"\\?\hid#vid_004c&pid_026c#1".to_string(),
            "Bluetooth".to_string(),
        )];
        state.sync_connected_devices(&devices);
        assert!(state.is_connected);
        assert_eq!(state.transport_name.as_deref(), Some("Bluetooth"));

        // Periodic battery sync
        state.battery_percent = Some(((i % 50) + 50) as u8);
        assert!(state.battery_percent.unwrap() >= 50);

        let tooltip = state.format_tooltip();
        assert!(tooltip.contains("Bluetooth"));
        assert!(tooltip.contains("Battery"));
    }
}

#[test]
fn test_soak_device_arrival_and_removal_loops() {
    let config = Config::default();
    let mut state = DaemonState::new(config, None);

    let usb_path = r"\\?\hid#vid_05ac&pid_026c&col01#1";
    let bt_path = r"\\?\hid#{00001124-0000-1000-8000-00805f9b34fb}_vid&0001004c_pid&026c#1";

    // 500 iterations of plug/unplug & switching
    for _ in 0..500 {
        // 1. USB connects
        let became_conn = state.on_device_arrival(usb_path, "USB");
        assert!(became_conn);
        assert!(state.is_connected);
        assert_eq!(state.transport_name.as_deref(), Some("USB"));

        // 2. Bluetooth also connects
        let second_conn = state.on_device_arrival(bt_path, "Bluetooth");
        assert!(!second_conn); // Already connected
        assert_eq!(state.connected_devices.len(), 2);

        // 3. USB disconnects (switching to pure BT)
        let total_disc = state.on_device_removal(usb_path);
        assert!(!total_disc); // Still connected via BT
        assert!(state.is_connected);
        assert_eq!(state.connected_devices.len(), 1);

        // 4. BT disconnects (all keyboards disconnected)
        let final_disc = state.on_device_removal(bt_path);
        assert!(final_disc);
        assert!(!state.is_connected);
        assert_eq!(state.connected_devices.len(), 0);
        assert_eq!(state.transport_name, None);
        assert_eq!(state.battery_percent, None);
    }
}

#[test]
fn test_soak_concurrent_profile_swaps_and_atomic_table() {
    let it_profile = NamedProfile::default_it();
    let us_profile = NamedProfile::default_us();

    let it_table = magicbridge_core::compile_profile(&it_profile);
    let us_table = magicbridge_core::compile_profile(&us_profile);

    let atomic_table = AtomicMappingTable::new(it_table.clone());

    let num_threads = 4;
    let iterations_per_thread = 500;
    let mut handles = Vec::new();

    for t in 0..num_threads {
        let table_clone = atomic_table.clone();
        let it_tbl = it_table.clone();
        let us_tbl = us_table.clone();

        handles.push(thread::spawn(move || {
            for i in 0..iterations_per_thread {
                if (t + i) % 2 == 0 {
                    table_clone.store(it_tbl.clone());
                } else {
                    table_clone.store(us_tbl.clone());
                }

                // Concurrent load
                let guard = table_clone.load();
                assert!(guard.fn_vk().is_some());
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let final_guard = atomic_table.load();
    assert!(final_guard.fn_vk().is_some());
}

#[test]
fn test_soak_logger_concurrent_stress() {
    logger::set_enabled(true);
    logger::clear_memory_logs();

    let num_threads = 4;
    let logs_per_thread = 100;
    let mut handles = Vec::new();

    for t in 0..num_threads {
        handles.push(thread::spawn(move || {
            for i in 0..logs_per_thread {
                logger::log("INFO", "soak_test", &format!("thread {t} msg {i}"));
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let recent = logger::get_recent_logs();
    assert!(recent.len() <= logger::MAX_MEMORY_LOG_ENTRIES);

    logger::set_enabled(false);
    logger::clear_memory_logs();
}
