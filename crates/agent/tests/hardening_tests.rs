//! Integration tests for Milestone 6: Hardening, Stuck-Key Resilience, Game Mode & Latency.
//!
//! Complies with F9.1..F9.2, NFR-P3, and NFR-S3.

use std::time::Instant;

use magicbridge_agent::{
    HookAction, KeyState, decode_remap, encode_remap, is_fn_down, is_fullscreen_rect,
    is_hook_active, is_ignored_shell_class, matches_process_list, process_keyboard_event,
    release_all_active_keys, set_active_table, set_hook_active,
};
use magicbridge_core::NamedProfile;
use magicbridge_core::engine::compile_profile;
use magicbridge_core::key::{Key, MAGICBRIDGE_EXTRA_INFO, VK_A, VK_F13, VK_LMENU, VK_RMENU};

const WM_KEYDOWN: u32 = 0x0100;

#[test]
fn test_emergency_key_release_on_pause() {
    let profile = NamedProfile::default_it();
    let table = compile_profile(&profile);
    set_active_table(table);
    set_hook_active(true);
    assert!(is_hook_active());

    // 1. Simulate Fn trigger key down
    let action_fn = process_keyboard_event(
        0,
        WM_KEYDOWN,
        VK_RMENU,
        0x38,
        0,
        Some(&compile_profile(&profile)),
        true,
    );
    assert_eq!(action_fn, HookAction::Swallow);
    assert!(is_fn_down());

    // 2. Pause hook (emergency disable)
    set_hook_active(false);
    assert!(!is_hook_active());
    assert!(!is_fn_down()); // Fn state must be immediately released (NFR-S3)

    // 3. Ensure events pass through when inactive
    let passthrough = process_keyboard_event(
        0,
        WM_KEYDOWN,
        VK_F13,
        0x64,
        0,
        Some(&compile_profile(&profile)),
        false,
    );
    assert_eq!(passthrough, HookAction::CallNextHook);

    // 4. Release all keys explicit call does not panic
    release_all_active_keys();
    assert!(!is_fn_down());
}

#[test]
fn test_stuck_key_resilience_rapid_toggling() {
    let profile = NamedProfile::default_it();
    let table = compile_profile(&profile);
    set_active_table(table.clone());

    // Simulate rapid typing and sudden pause/unpause 500 times
    for i in 0..500 {
        set_hook_active(true);

        // Process a remapped keystroke
        let _ = process_keyboard_event(0, WM_KEYDOWN, VK_F13, 0x64, 0, Some(&table), true);

        // Suddenly toggle active state off (e.g. game mode or global pause hotkey)
        set_hook_active(false);
        assert!(!is_hook_active());
        assert!(!is_fn_down());

        if i % 100 == 0 {
            release_all_active_keys();
        }
    }
}

#[test]
fn test_game_mode_auto_pause_process_and_window_detection() {
    let processes = vec![
        "Cyberpunk2077.exe".to_string(),
        "Starfield.exe".to_string(),
        "dota2".to_string(),
    ];

    // Case insensitivity & extension flexibility
    assert!(matches_process_list("Cyberpunk2077.exe", &processes));
    assert!(matches_process_list("cyberpunk2077.exe", &processes));
    assert!(matches_process_list("Cyberpunk2077", &processes));
    assert!(matches_process_list("starfield", &processes));
    assert!(matches_process_list("Dota2.exe", &processes));
    assert!(!matches_process_list("notepad.exe", &processes));
    assert!(!matches_process_list("explorer.exe", &processes));

    // Fullscreen rect checks
    assert!(is_fullscreen_rect(0, 0, 1920, 1080, 1920, 1080));
    assert!(is_fullscreen_rect(-2, -2, 1922, 1082, 1920, 1080)); // borderless fullscreen
    assert!(!is_fullscreen_rect(100, 100, 1000, 800, 1920, 1080)); // windowed

    // Ignored shell classes
    assert!(is_ignored_shell_class("Progman"));
    assert!(is_ignored_shell_class("WorkerW"));
    assert!(is_ignored_shell_class("Shell_TrayWnd"));
    assert!(is_ignored_shell_class("Windows.UI.Core.CoreWindow"));
    assert!(!is_ignored_shell_class("UnrealWindow"));
}

#[test]
fn test_nfr_p3_hook_latency_budget() {
    let profile = NamedProfile::default_it();
    let table = compile_profile(&profile);
    set_active_table(table.clone());
    set_hook_active(true);

    let iterations = 50_000;

    // Warm-up
    for _ in 0..1_000 {
        let _ = process_keyboard_event(0, WM_KEYDOWN, VK_A, 0x1E, 0, Some(&table), true);
    }

    let start = Instant::now();

    for i in 0..iterations {
        let (vk, sc) = match i % 4 {
            0 => (VK_A, 0x1E),                          // unmapped pass-through
            1 => (VK_F13, 0x64),                        // remapped key (F13 -> PrtSc)
            2 => (VK_LMENU, 0x38),                      // swapped modifier
            _ => (VK_A, MAGICBRIDGE_EXTRA_INFO as u32), // self-injection fast-path
        };

        let dw_extra = if i % 4 == 3 {
            MAGICBRIDGE_EXTRA_INFO
        } else {
            0
        };
        let _ = process_keyboard_event(0, WM_KEYDOWN, vk, sc, dw_extra, Some(&table), true);
    }

    let total_elapsed = start.elapsed();
    let avg_ns_per_keystroke = total_elapsed.as_nanos() / (iterations as u128);

    println!(
        "[NFR-P3 Benchmark] Processed {iterations} keystrokes in {:?}. Average latency: {avg_ns_per_keystroke} ns/stroke",
        total_elapsed
    );

    // Hard budget per NFR-P3: < 0.1 ms (100,000 ns). Target: < 0.01 ms (10,000 ns).
    assert!(
        avg_ns_per_keystroke < 100_000,
        "Hook latency exceeded NFR-P3 budget of 0.1ms: {avg_ns_per_keystroke} ns"
    );
}

#[test]
fn test_encode_decode_remap_roundtrip() {
    let key = Key::new(0x2C, 0x137, true); // PrintScreen extended
    let encoded = encode_remap(key);
    let decoded = decode_remap(encoded);

    match decoded {
        KeyState::Remapped(k) => {
            assert_eq!(k.vk, 0x2C);
            assert_eq!(k.scan_code, 0x137);
            assert!(k.extended);
        }
        _ => panic!("Expected KeyState::Remapped"),
    }

    assert_eq!(decode_remap(0), KeyState::None);
    assert_eq!(decode_remap(1), KeyState::Swallowed);
}
