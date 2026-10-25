//! Physical layout model and visual key representations for Apple Magic Keyboard (A1843).
//!
//! Provides coordinate geometry, key clusters (Main typing block, Navigation cluster,
//! Numeric keypad cluster), and localized key legends across ANSI and ISO layouts (F7.2, F7.4).

use magicbridge_core::KeyboardLayout;
use magicbridge_core::key::*;

/// Cluster on the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCluster {
    /// Main alphanumeric typing block.
    Main,
    /// Navigation & editing cluster (Home/End/PgUp/PgDn, Arrows, F13-F15).
    Navigation,
    /// Numeric keypad cluster (Numpad digits, arithmetic ops, F16-F19).
    Numpad,
}

/// A physical key on the keyboard with position and dimensions in standard unit keys (1u = 1 standard key width).
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicalKey {
    /// Unique identifier string (e.g. "Escape", "F13", "Space", "RightOption").
    pub id: &'static str,
    /// Default label.
    pub default_label: &'static str,
    /// Windows Virtual Key code.
    pub vk: u16,
    /// Vertical row (0 = F-row, 1 = numbers, 2 = QWERTY, 3 = ASDF, 4 = ZXCV, 5 = space/mods).
    pub row: usize,
    /// Horizontal offset in unit keys (1u = standard square key).
    pub x: f32,
    /// Key width in unit keys.
    pub width: f32,
    /// Key height in unit keys.
    pub height: f32,
    /// Key cluster.
    pub cluster: KeyCluster,
}

/// Represents the visual A1843 keyboard geometry.
#[derive(Debug, Clone)]
pub struct A1843Model {
    pub keys: Vec<PhysicalKey>,
}

impl Default for A1843Model {
    fn default() -> Self {
        Self::new()
    }
}

impl A1843Model {
    /// Constructs the complete A1843 physical key collection.
    #[must_use]
    pub fn new() -> Self {
        let mut keys = Vec::with_capacity(110);

        // ====================================================================
        // ROW 0: Top Function Row (F-keys and F13..F19)
        // ====================================================================
        // Main block: Esc, F1..F12
        keys.push(PhysicalKey {
            id: "Escape",
            default_label: "esc",
            vk: VK_ESCAPE,
            row: 0,
            x: 0.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F1",
            default_label: "F1",
            vk: VK_F1,
            row: 0,
            x: 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F2",
            default_label: "F2",
            vk: VK_F2,
            row: 0,
            x: 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F3",
            default_label: "F3",
            vk: VK_F3,
            row: 0,
            x: 3.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F4",
            default_label: "F4",
            vk: VK_F4,
            row: 0,
            x: 4.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F5",
            default_label: "F5",
            vk: VK_F5,
            row: 0,
            x: 5.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F6",
            default_label: "F6",
            vk: VK_F6,
            row: 0,
            x: 6.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F7",
            default_label: "F7",
            vk: VK_F7,
            row: 0,
            x: 7.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F8",
            default_label: "F8",
            vk: VK_F8,
            row: 0,
            x: 8.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F9",
            default_label: "F9",
            vk: VK_F9,
            row: 0,
            x: 9.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F10",
            default_label: "F10",
            vk: VK_F10,
            row: 0,
            x: 10.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F11",
            default_label: "F11",
            vk: VK_F11,
            row: 0,
            x: 11.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "F12",
            default_label: "F12",
            vk: VK_F12,
            row: 0,
            x: 12.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Navigation top row: F13..F15 (Offset x: 15.5)
        let nav_x = 15.5;
        keys.push(PhysicalKey {
            id: "F13",
            default_label: "F13",
            vk: VK_F13,
            row: 0,
            x: nav_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "F14",
            default_label: "F14",
            vk: VK_F14,
            row: 0,
            x: nav_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "F15",
            default_label: "F15",
            vk: VK_F15,
            row: 0,
            x: nav_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });

        // Numpad top row: F16..F19 (Offset x: 19.0)
        let num_x = 19.0;
        keys.push(PhysicalKey {
            id: "F16",
            default_label: "F16",
            vk: VK_F16,
            row: 0,
            x: num_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "F17",
            default_label: "F17",
            vk: VK_F17,
            row: 0,
            x: num_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "F18",
            default_label: "F18",
            vk: VK_F18,
            row: 0,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "F19",
            default_label: "F19",
            vk: VK_F19,
            row: 0,
            x: num_x + 3.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });

        // ====================================================================
        // ROW 1: Number Row (~ / 1..0 / Backspace)
        // ====================================================================
        keys.push(PhysicalKey {
            id: "Grave",
            default_label: "~",
            vk: VK_OEM_3,
            row: 1,
            x: 0.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        let digits = [
            ("1", VK_1),
            ("2", VK_2),
            ("3", VK_3),
            ("4", VK_4),
            ("5", VK_5),
            ("6", VK_6),
            ("7", VK_7),
            ("8", VK_8),
            ("9", VK_9),
            ("0", VK_0),
        ];
        for (i, (lbl, vk)) in digits.into_iter().enumerate() {
            keys.push(PhysicalKey {
                id: lbl,
                default_label: lbl,
                vk,
                row: 1,
                x: 1.0 + i as f32,
                width: 1.0,
                height: 1.0,
                cluster: KeyCluster::Main,
            });
        }
        keys.push(PhysicalKey {
            id: "Minus",
            default_label: "-",
            vk: VK_OEM_MINUS,
            row: 1,
            x: 11.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Equals",
            default_label: "=",
            vk: VK_OEM_PLUS,
            row: 1,
            x: 12.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Backspace",
            default_label: "delete ⌫",
            vk: VK_BACK,
            row: 1,
            x: 13.0,
            width: 2.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Nav row 1: Fn key (Mac physical label; stripped by kbdhid.sys in kernel mode)
        keys.push(PhysicalKey {
            id: "Fn",
            default_label: "fn",
            vk: 0,
            row: 1,
            x: nav_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "Home",
            default_label: "home ↖",
            vk: VK_HOME,
            row: 1,
            x: nav_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "PageUp",
            default_label: "page up ⇞",
            vk: VK_PRIOR,
            row: 1,
            x: nav_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });

        // Numpad row 1: Clear, =, /, *
        keys.push(PhysicalKey {
            id: "Clear",
            default_label: "clear ⌧",
            vk: VK_NUMLOCK,
            row: 1,
            x: num_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumEquals",
            default_label: "=",
            vk: VK_OEM_PLUS,
            row: 1,
            x: num_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumDivide",
            default_label: "/",
            vk: VK_DIVIDE,
            row: 1,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumMultiply",
            default_label: "*",
            vk: VK_MULTIPLY,
            row: 1,
            x: num_x + 3.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });

        // ====================================================================
        // ROW 2: QWERTY Row (Tab, Q..P, [, ], \)
        // ====================================================================
        keys.push(PhysicalKey {
            id: "Tab",
            default_label: "tab ⇥",
            vk: VK_TAB,
            row: 2,
            x: 0.0,
            width: 1.5,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        let row2_letters = [
            ("Q", VK_Q),
            ("W", VK_W),
            ("E", VK_E),
            ("R", VK_R),
            ("T", VK_T),
            ("Y", VK_Y),
            ("U", VK_U),
            ("I", VK_I),
            ("O", VK_O),
            ("P", VK_P),
        ];
        for (i, (lbl, vk)) in row2_letters.into_iter().enumerate() {
            keys.push(PhysicalKey {
                id: lbl,
                default_label: lbl,
                vk,
                row: 2,
                x: 1.5 + i as f32,
                width: 1.0,
                height: 1.0,
                cluster: KeyCluster::Main,
            });
        }
        keys.push(PhysicalKey {
            id: "LBracket",
            default_label: "[",
            vk: VK_OEM_4,
            row: 2,
            x: 11.5,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "RBracket",
            default_label: "]",
            vk: VK_OEM_6,
            row: 2,
            x: 12.5,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Backslash",
            default_label: "\\",
            vk: VK_OEM_5,
            row: 2,
            x: 13.5,
            width: 1.5,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Nav row 2: Forward Delete, End, Page Down
        keys.push(PhysicalKey {
            id: "Delete",
            default_label: "⌦",
            vk: VK_DELETE,
            row: 2,
            x: nav_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "End",
            default_label: "end ↘",
            vk: VK_END,
            row: 2,
            x: nav_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "PageDown",
            default_label: "page down ⇟",
            vk: VK_NEXT,
            row: 2,
            x: nav_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });

        // Numpad row 2: 7, 8, 9, -
        keys.push(PhysicalKey {
            id: "Num7",
            default_label: "7",
            vk: VK_NUMPAD7,
            row: 2,
            x: num_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num8",
            default_label: "8",
            vk: VK_NUMPAD8,
            row: 2,
            x: num_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num9",
            default_label: "9",
            vk: VK_NUMPAD9,
            row: 2,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumSubtract",
            default_label: "-",
            vk: VK_SUBTRACT,
            row: 2,
            x: num_x + 3.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });

        // ====================================================================
        // ROW 3: Home Row (Caps Lock, A..L, ;, ', Enter)
        // ====================================================================
        keys.push(PhysicalKey {
            id: "CapsLock",
            default_label: "caps lock ⇪",
            vk: VK_CAPITAL,
            row: 3,
            x: 0.0,
            width: 1.75,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        let row3_letters = [
            ("A", VK_A),
            ("S", VK_S),
            ("D", VK_D),
            ("F", VK_F),
            ("G", VK_G),
            ("H", VK_H),
            ("J", VK_J),
            ("K", VK_K),
            ("L", VK_L),
        ];
        for (i, (lbl, vk)) in row3_letters.into_iter().enumerate() {
            keys.push(PhysicalKey {
                id: lbl,
                default_label: lbl,
                vk,
                row: 3,
                x: 1.75 + i as f32,
                width: 1.0,
                height: 1.0,
                cluster: KeyCluster::Main,
            });
        }
        keys.push(PhysicalKey {
            id: "Semicolon",
            default_label: ";",
            vk: VK_OEM_1,
            row: 3,
            x: 10.75,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Quote",
            default_label: "'",
            vk: VK_OEM_7,
            row: 3,
            x: 11.75,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Enter",
            default_label: "return ↩",
            vk: VK_RETURN,
            row: 3,
            x: 12.75,
            width: 2.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Numpad row 3: 4, 5, 6, +
        keys.push(PhysicalKey {
            id: "Num4",
            default_label: "4",
            vk: VK_NUMPAD4,
            row: 3,
            x: num_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num5",
            default_label: "5",
            vk: VK_NUMPAD5,
            row: 3,
            x: num_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num6",
            default_label: "6",
            vk: VK_NUMPAD6,
            row: 3,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumAdd",
            default_label: "+",
            vk: VK_ADD,
            row: 3,
            x: num_x + 3.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });

        // ====================================================================
        // ROW 4: Shift Row (Left Shift, ISO <>, Z..M, ,, ., /, Right Shift)
        // ====================================================================
        keys.push(PhysicalKey {
            id: "LeftShift",
            default_label: "shift ⇧",
            vk: VK_LSHIFT,
            row: 4,
            x: 0.0,
            width: 1.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "IsoLessGreater",
            default_label: "< >",
            vk: VK_OEM_102,
            row: 4,
            x: 1.25,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        let row4_letters = [
            ("Z", VK_Z),
            ("X", VK_X),
            ("C", VK_C),
            ("V", VK_V),
            ("B", VK_B),
            ("N", VK_N),
            ("M", VK_M),
        ];
        for (i, (lbl, vk)) in row4_letters.into_iter().enumerate() {
            keys.push(PhysicalKey {
                id: lbl,
                default_label: lbl,
                vk,
                row: 4,
                x: 2.25 + i as f32,
                width: 1.0,
                height: 1.0,
                cluster: KeyCluster::Main,
            });
        }
        keys.push(PhysicalKey {
            id: "Comma",
            default_label: ",",
            vk: VK_OEM_COMMA,
            row: 4,
            x: 9.25,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Period",
            default_label: ".",
            vk: VK_OEM_PERIOD,
            row: 4,
            x: 10.25,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Slash",
            default_label: "/",
            vk: VK_OEM_2,
            row: 4,
            x: 11.25,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "RightShift",
            default_label: "shift ⇧",
            vk: VK_RSHIFT,
            row: 4,
            x: 12.25,
            width: 2.75,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Nav row 4: Up Arrow (centered at col 1)
        keys.push(PhysicalKey {
            id: "Up",
            default_label: "↑",
            vk: VK_UP,
            row: 4,
            x: nav_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });

        // Numpad row 4: 1, 2, 3, Enter
        keys.push(PhysicalKey {
            id: "Num1",
            default_label: "1",
            vk: VK_NUMPAD1,
            row: 4,
            x: num_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num2",
            default_label: "2",
            vk: VK_NUMPAD2,
            row: 4,
            x: num_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "Num3",
            default_label: "3",
            vk: VK_NUMPAD3,
            row: 4,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumEnter",
            default_label: "enter ⌤",
            vk: VK_RETURN,
            row: 4,
            x: num_x + 3.0,
            width: 1.0,
            height: 2.0,
            cluster: KeyCluster::Numpad,
        });

        // ====================================================================
        // ROW 5: Modifier Row (Ctrl, Option, Cmd, Spacebar, Cmd, Option, Ctrl)
        // ====================================================================
        keys.push(PhysicalKey {
            id: "LeftControl",
            default_label: "control ⌃",
            vk: VK_LCONTROL,
            row: 5,
            x: 0.0,
            width: 1.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "LeftOption",
            default_label: "option ⌥",
            vk: VK_LMENU,
            row: 5,
            x: 1.25,
            width: 1.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "LeftCommand",
            default_label: "command ⌘",
            vk: VK_LWIN,
            row: 5,
            x: 2.5,
            width: 1.5,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "Space",
            default_label: "space",
            vk: VK_SPACE,
            row: 5,
            x: 4.0,
            width: 6.0,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "RightCommand",
            default_label: "command ⌘",
            vk: VK_RWIN,
            row: 5,
            x: 10.0,
            width: 1.5,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "RightOption",
            default_label: "option ⌥",
            vk: VK_RMENU,
            row: 5,
            x: 11.5,
            width: 1.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });
        keys.push(PhysicalKey {
            id: "RightControl",
            default_label: "control ⌃",
            vk: VK_RCONTROL,
            row: 5,
            x: 12.75,
            width: 1.25,
            height: 1.0,
            cluster: KeyCluster::Main,
        });

        // Nav row 5: Left, Down, Right arrows
        keys.push(PhysicalKey {
            id: "Left",
            default_label: "←",
            vk: VK_LEFT,
            row: 5,
            x: nav_x,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "Down",
            default_label: "↓",
            vk: VK_DOWN,
            row: 5,
            x: nav_x + 1.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });
        keys.push(PhysicalKey {
            id: "Right",
            default_label: "→",
            vk: VK_RIGHT,
            row: 5,
            x: nav_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Navigation,
        });

        // Numpad row 5: 0 (2u wide), . (decimal)
        keys.push(PhysicalKey {
            id: "Num0",
            default_label: "0",
            vk: VK_NUMPAD0,
            row: 5,
            x: num_x,
            width: 2.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });
        keys.push(PhysicalKey {
            id: "NumDecimal",
            default_label: ".",
            vk: VK_DECIMAL,
            row: 5,
            x: num_x + 2.0,
            width: 1.0,
            height: 1.0,
            cluster: KeyCluster::Numpad,
        });

        Self { keys }
    }

    /// Finds a physical key by its unique ID string.
    #[must_use]
    pub fn find_by_id(&self, id: &str) -> Option<&PhysicalKey> {
        self.keys.iter().find(|k| k.id.eq_ignore_ascii_case(id))
    }

    /// Finds a physical key matching a Windows Virtual Key code.
    #[must_use]
    pub fn find_by_vk(&self, vk: u16) -> Option<&PhysicalKey> {
        self.keys.iter().find(|k| k.vk == vk)
    }

    /// Returns the localized visual legend for a key given the active `KeyboardLayout`.
    #[must_use]
    pub fn get_key_label(&self, key_id: &str, layout: &KeyboardLayout) -> String {
        match layout {
            KeyboardLayout::IsoItalian => Self::label_italian(key_id),
            KeyboardLayout::AnsiUs => Self::label_us(key_id),
            KeyboardLayout::IsoUk => Self::label_uk(key_id),
            KeyboardLayout::IsoGerman => Self::label_german(key_id),
            KeyboardLayout::IsoFrench => Self::label_french(key_id),
            KeyboardLayout::Custom(_) => Self::label_italian(key_id),
        }
    }

    fn label_italian(id: &str) -> String {
        match id {
            "Grave" => "\\ |".to_string(),
            "1" => "1 !".to_string(),
            "2" => "2 \"".to_string(),
            "3" => "3 £".to_string(),
            "4" => "4 $".to_string(),
            "5" => "5 %".to_string(),
            "6" => "6 &".to_string(),
            "7" => "7 /".to_string(),
            "8" => "8 (".to_string(),
            "9" => "9 )".to_string(),
            "0" => "0 =".to_string(),
            "Minus" => "' ?".to_string(),
            "Equals" => "ì ^".to_string(),
            "LBracket" => "è é [".to_string(),
            "RBracket" => "+ * ]".to_string(),
            "Backslash" => "ù §".to_string(),
            "Semicolon" => "ò ç @".to_string(),
            "Quote" => "à ° #".to_string(),
            "IsoLessGreater" => "< >".to_string(),
            "Comma" => ", ;".to_string(),
            "Period" => ". :".to_string(),
            "Slash" => "- _".to_string(),
            _ => id.to_string(),
        }
    }

    fn label_us(id: &str) -> String {
        match id {
            "Grave" => "` ~".to_string(),
            "1" => "1 !".to_string(),
            "2" => "2 @".to_string(),
            "3" => "3 #".to_string(),
            "4" => "4 $".to_string(),
            "5" => "5 %".to_string(),
            "6" => "6 ^".to_string(),
            "7" => "7 &".to_string(),
            "8" => "8 *".to_string(),
            "9" => "9 (".to_string(),
            "0" => "0 )".to_string(),
            "Minus" => "- _".to_string(),
            "Equals" => "= +".to_string(),
            "LBracket" => "[ {".to_string(),
            "RBracket" => "] }".to_string(),
            "Backslash" => "\\ |".to_string(),
            "Semicolon" => "; :".to_string(),
            "Quote" => "' \"".to_string(),
            "IsoLessGreater" => "".to_string(),
            "Comma" => ", <".to_string(),
            "Period" => ". >".to_string(),
            "Slash" => "/ ?".to_string(),
            _ => id.to_string(),
        }
    }

    fn label_uk(id: &str) -> String {
        match id {
            "Grave" => "` ¬".to_string(),
            "2" => "2 \"".to_string(),
            "3" => "3 £".to_string(),
            "Minus" => "- _".to_string(),
            "Equals" => "= +".to_string(),
            "Quote" => "' @".to_string(),
            "Backslash" => "# ~".to_string(),
            _ => Self::label_us(id),
        }
    }

    fn label_german(id: &str) -> String {
        match id {
            "Grave" => "^ °".to_string(),
            "Z" => "Y".to_string(),
            "Y" => "Z".to_string(),
            "LBracket" => "Ü".to_string(),
            "RBracket" => "+ *".to_string(),
            "Semicolon" => "Ö".to_string(),
            "Quote" => "Ä".to_string(),
            "Minus" => "ß ?".to_string(),
            _ => id.to_string(),
        }
    }

    fn label_french(id: &str) -> String {
        match id {
            "A" => "Q".to_string(),
            "Q" => "A".to_string(),
            "Z" => "W".to_string(),
            "W" => "Z".to_string(),
            "M" => ", ?".to_string(),
            "Comma" => "; .".to_string(),
            _ => id.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a1843_model_key_count_and_clusters() {
        let model = A1843Model::new();
        // A1843 has approximately 109 physical keys
        assert!(
            model.keys.len() >= 105,
            "Expected ~109 keys, got {}",
            model.keys.len()
        );

        let f13 = model.find_by_id("F13").expect("F13 must exist");
        assert_eq!(f13.vk, VK_F13);
        assert_eq!(f13.cluster, KeyCluster::Navigation);

        let clear = model.find_by_id("Clear").expect("Clear must exist");
        assert_eq!(clear.vk, VK_NUMLOCK);
        assert_eq!(clear.cluster, KeyCluster::Numpad);

        let f19 = model.find_by_id("F19").expect("F19 must exist");
        assert_eq!(f19.vk, VK_F19);
        assert_eq!(f19.cluster, KeyCluster::Numpad);
    }

    #[test]
    fn test_a1843_model_localized_legends() {
        let model = A1843Model::new();

        // Italian legends
        assert_eq!(
            model.get_key_label("Grave", &KeyboardLayout::IsoItalian),
            "\\ |"
        );
        assert_eq!(
            model.get_key_label("Quote", &KeyboardLayout::IsoItalian),
            "à ° #"
        );

        // US legends
        assert_eq!(model.get_key_label("Grave", &KeyboardLayout::AnsiUs), "` ~");
        assert_eq!(model.get_key_label("2", &KeyboardLayout::AnsiUs), "2 @");

        // German QWERTZ swap
        assert_eq!(model.get_key_label("Z", &KeyboardLayout::IsoGerman), "Y");
        assert_eq!(model.get_key_label("Y", &KeyboardLayout::IsoGerman), "Z");

        // French AZERTY swap
        assert_eq!(model.get_key_label("A", &KeyboardLayout::IsoFrench), "Q");
        assert_eq!(model.get_key_label("Q", &KeyboardLayout::IsoFrench), "A");
    }

    #[test]
    fn test_find_by_vk() {
        let model = A1843Model::new();
        let key = model.find_by_vk(VK_ESCAPE).expect("Escape found by VK");
        assert_eq!(key.id, "Escape");

        let key = model.find_by_vk(VK_RMENU).expect("RightOption found by VK");
        assert_eq!(key.id, "RightOption");
    }
}
