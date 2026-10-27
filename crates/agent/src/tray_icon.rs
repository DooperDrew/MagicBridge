//! Dynamic tray battery badge renderer using GDI off-screen bitmaps (F6.1, IMP-04).
//!
//! Renders live battery percentage, charge status (charging/discharging/AC),
//! connection state, and pause/game-mode indicators dynamically onto a 32-bit
//! ARGB icon for the Windows system notification area.

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateBitmap, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, HICON, ICONINFO, SM_CXSMICON, SM_CYSMICON,
};

use crate::state::DaemonState;

/// 3x5 bitmap font representation for digits '0'..'9'.
/// Each entry is 5 rows, with bits 2, 1, 0 representing pixels from left to right.
const DIGIT_FONT_3X5: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

/// In-memory 32-bit ARGB pixel canvas.
#[derive(Debug, Clone)]
pub struct RgbaCanvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl RgbaCanvas {
    /// Creates a blank transparent canvas with dimensions `width x height`.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0u32; width * height],
        }
    }

    /// Sets a pixel at `(x, y)` to an ARGB color value.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
        }
    }

    /// Returns the pixel at `(x, y)`.
    #[must_use]
    pub fn get_pixel(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            0
        }
    }

    /// Fills an axis-aligned rectangle from `[x0..=x1, y0..=y1]`.
    pub fn fill_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
        let max_x = x1.min(self.width.saturating_sub(1));
        let max_y = y1.min(self.height.saturating_sub(1));
        for y in y0..=max_y {
            for x in x0..=max_x {
                self.pixels[y * self.width + x] = color;
            }
        }
    }

    /// Draws a 1-pixel wide rectangular outline from `[x0..=x1, y0..=y1]`.
    pub fn draw_rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: u32) {
        let max_x = x1.min(self.width.saturating_sub(1));
        let max_y = y1.min(self.height.saturating_sub(1));
        for x in x0..=max_x {
            self.set_pixel(x, y0, color);
            self.set_pixel(x, max_y, color);
        }
        for y in y0..=max_y {
            self.set_pixel(x0, y, color);
            self.set_pixel(max_x, y, color);
        }
    }

    /// Draws a single digit (0..=9) using the 3x5 font at `(x, y)`.
    pub fn draw_digit(&mut self, x: usize, y: usize, digit: u8, color: u32) {
        if let Some(rows) = DIGIT_FONT_3X5.get(digit as usize) {
            for (row_idx, &row_bits) in rows.iter().enumerate() {
                for col_idx in 0..3 {
                    if (row_bits & (1 << (2 - col_idx))) != 0 {
                        self.set_pixel(x + col_idx, y + row_idx, color);
                    }
                }
            }
        }
    }

    /// Draws a 1..3 digit percentage number (0..=100) centered horizontally at `center_x`.
    /// Includes a subtle 1-pixel dark shadow underneath for high contrast on light backgrounds.
    pub fn draw_percentage(&mut self, center_x: usize, y: usize, num: u8, color: u32) {
        let shadow_color = 0xFF0A0F1D; // Dark navy shadow

        let (total_width, is_hundred) = if num >= 100 {
            (9, true)
        } else if num >= 10 {
            (7, false)
        } else {
            (3, false)
        };

        let start_x = center_x.saturating_sub(total_width / 2);

        // Draw shadow first (offset +1,+1)
        self.render_digits_at(start_x + 1, y + 1, num, is_hundred, shadow_color);
        // Draw foreground digits
        self.render_digits_at(start_x, y, num, is_hundred, color);
    }

    fn render_digits_at(
        &mut self,
        start_x: usize,
        y: usize,
        num: u8,
        is_hundred: bool,
        color: u32,
    ) {
        if is_hundred {
            // "100" condensed: 1px for '1', 1px space, 3px for '0', 1px space, 3px for '0'
            for row in 0..5 {
                self.set_pixel(start_x, y + row, color);
            }
            self.draw_digit(start_x + 2, y, 0, color);
            self.draw_digit(start_x + 6, y, 0, color);
        } else if num >= 10 {
            let tens = num / 10;
            let ones = num % 10;
            self.draw_digit(start_x, y, tens, color);
            self.draw_digit(start_x + 4, y, ones, color);
        } else {
            self.draw_digit(start_x, y, num, color);
        }
    }

    /// Draws a charging lightning bolt glyph centered at `(cx, cy)`.
    pub fn draw_lightning_bolt(&mut self, cx: usize, cy: usize, color: u32) {
        let shadow = 0xFF0F172A;
        // 5x7 lightning bolt offsets: (dx, dy)
        let bolt_pixels: &[(i32, i32)] = &[
            (0, -3),
            (1, -3),
            (-1, -2),
            (0, -2),
            (-2, -1),
            (-1, -1),
            (0, -1),
            (-3, 0),
            (-2, 0),
            (-1, 0),
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (1, 1),
            (0, 2),
            (1, 2),
            (0, 3),
        ];

        // Draw shadow/outline
        for &(dx, dy) in bolt_pixels {
            let px = (cx as i32 + dx + 1) as usize;
            let py = (cy as i32 + dy + 1) as usize;
            self.set_pixel(px, py, shadow);
        }
        // Draw foreground bolt
        for &(dx, dy) in bolt_pixels {
            let px = (cx as i32 + dx) as usize;
            let py = (cy as i32 + dy) as usize;
            self.set_pixel(px, py, color);
        }
    }
}

/// Renders a dynamic status badge canvas according to the current daemon state.
#[must_use]
pub fn render_canvas(width: usize, height: usize, state: &DaemonState) -> RgbaCanvas {
    let mut canvas = RgbaCanvas::new(width, height);

    let border_color = 0xFF1E293B; // Slate dark border
    let shadow_color = 0x80000000; // 50% black shadow

    if !state.is_connected {
        // Disconnected: Render sleek gray keyboard frame with small red dot
        let k_w = width.saturating_sub(4);
        let k_h = (height * 10) / 16;
        let k_x = 2;
        let k_y = (height - k_h) / 2;

        // Shadow & frame
        canvas.fill_rect(k_x + 1, k_y + 1, k_x + k_w, k_y + k_h, shadow_color);
        canvas.fill_rect(k_x, k_y, k_x + k_w - 1, k_y + k_h - 1, 0xFF475569);
        canvas.draw_rect(k_x, k_y, k_x + k_w - 1, k_y + k_h - 1, border_color);

        // Key grid dots
        let dot_color = 0xFF94A3B8;
        for dy in [2, 4, 6] {
            for dx in [2, 5, 8, 11] {
                if k_x + dx < k_x + k_w - 2 && k_y + dy < k_y + k_h - 2 {
                    canvas.set_pixel(k_x + dx, k_y + dy, dot_color);
                }
            }
        }

        // Small disconnected red dot in bottom-right corner
        let dot_x = width.saturating_sub(4);
        let dot_y = height.saturating_sub(4);
        canvas.fill_rect(dot_x, dot_y, dot_x + 2, dot_y + 2, 0xFFEF4444);
        return canvas;
    }

    // Connected: Render battery gauge and percentage
    let pct = state.battery_percent.unwrap_or(100);

    // Color based on battery level and thresholds
    let fill_color = if pct <= state.config.low_battery_threshold_percent {
        0xFFEF4444 // Red (< threshold)
    } else if pct <= 25 {
        0xFFF59E0B // Amber caution
    } else {
        0xFF22C55E // Vibrant green
    };

    // Battery geometry (Top area)
    let b_x = 1;
    let b_y = 1;
    let b_w = width.saturating_sub(4);
    let b_h = ((height * 7) / 16).max(5);

    // Battery body background & border
    canvas.fill_rect(b_x + 1, b_y + 1, b_x + b_w, b_y + b_h, shadow_color);
    canvas.fill_rect(b_x, b_y, b_x + b_w - 1, b_y + b_h - 1, 0xFF0F172A);
    canvas.draw_rect(b_x, b_y, b_x + b_w - 1, b_y + b_h - 1, border_color);

    // Terminal nub on the right
    let nub_x = b_x + b_w;
    let nub_y_start = b_y + (b_h / 3);
    let nub_h = (b_h / 3).max(2);
    canvas.fill_rect(
        nub_x,
        nub_y_start,
        nub_x + 1,
        nub_y_start + nub_h - 1,
        border_color,
    );

    // Battery fill level
    let inner_w = b_w.saturating_sub(2);
    let inner_h = b_h.saturating_sub(2);
    let fill_w = if pct == 0 {
        0
    } else {
        ((inner_w * pct as usize) + 50) / 100
    }
    .clamp(if pct > 0 { 1 } else { 0 }, inner_w);

    if fill_w > 0 && inner_h > 0 {
        canvas.fill_rect(b_x + 1, b_y + 1, b_x + fill_w, b_y + inner_h, fill_color);
    }

    // Charging indicator (Lightning bolt)
    if state.is_charging {
        let cx = b_x + (b_w / 2);
        let cy = b_y + (b_h / 2);
        canvas.draw_lightning_bolt(cx, cy, 0xFFFACC15); // Electric yellow
    }

    // Bottom area: Percentage number or pause badge
    if !state.enabled || state.game_mode_active {
        // Paused: Draw two amber pause bars in the center
        let p_y = b_y + b_h + 2;
        let cx = width / 2;
        let bar_h = 5;
        canvas.fill_rect(cx - 3, p_y, cx - 2, p_y + bar_h, 0xFFF59E0B);
        canvas.fill_rect(cx + 1, p_y, cx + 2, p_y + bar_h, 0xFFF59E0B);
    } else {
        // Numeric percentage
        let text_y = b_y + b_h + 2;
        let cx = width / 2;
        let text_color = 0xFFFFFFFF; // Pure white
        canvas.draw_percentage(cx, text_y, pct, text_color);
    }

    canvas
}

/// Dynamically renders and builds a Win32 `HICON` representing the current daemon battery status.
///
/// Returns `Ok(HICON)` on success, which the caller owns and MUST release via `DestroyIcon`.
pub fn create_battery_tray_icon(state: &DaemonState) -> windows::core::Result<HICON> {
    // Determine the system small icon metrics (typically 16x16, 20x20, 24x24, or 32x32 depending on DPI).
    // SAFETY: GetSystemMetrics has no preconditions.
    let cx = unsafe { GetSystemMetrics(SM_CXSMICON) }.clamp(16, 64) as usize;
    // SAFETY: GetSystemMetrics has no preconditions.
    let cy = unsafe { GetSystemMetrics(SM_CYSMICON) }.clamp(16, 64) as usize;

    let canvas = render_canvas(cx, cy, state);
    create_hicon_from_canvas(&canvas)
}

/// Creates a 32-bit ARGB `HICON` from an `RgbaCanvas` using Windows GDI off-screen bitmaps.
pub fn create_hicon_from_canvas(canvas: &RgbaCanvas) -> windows::core::Result<HICON> {
    let width = canvas.width as i32;
    let height = canvas.height as i32;

    // SAFETY: CreateCompatibleDC with None creates a memory DC compatible with the display.
    let mem_dc = unsafe { CreateCompatibleDC(None) };
    if mem_dc.is_invalid() {
        return Err(windows::core::Error::from_thread());
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // Negative height indicates top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    // SAFETY: CreateDIBSection allocates a 32-bit DIB section and initializes bits_ptr.
    let hbm_color =
        unsafe { CreateDIBSection(Some(mem_dc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0)? };

    if !bits_ptr.is_null() {
        // SAFETY: bits_ptr points to `width * height * 4` bytes allocated by CreateDIBSection.
        unsafe {
            std::ptr::copy_nonoverlapping(
                canvas.pixels.as_ptr() as *const u8,
                bits_ptr as *mut u8,
                canvas.pixels.len() * 4,
            );
        }
    }

    // A 1-bpp monochrome mask bitmap of identical dimensions is required by CreateIconIndirect.
    // For 32-bit alpha-channel icons, Windows Shell ignores mask bits where alpha != 0.
    // SAFETY: CreateBitmap creates a valid 1-bpp bitmap.
    let hbm_mask = unsafe { CreateBitmap(width, height, 1, 1, None) };

    let icon_info = ICONINFO {
        fIcon: true.into(),
        xHotspot: 0,
        yHotspot: 0,
        hbmMask: hbm_mask,
        hbmColor: hbm_color,
    };

    // SAFETY: CreateIconIndirect creates an HICON from the supplied color and mask bitmaps.
    let hicon_res = unsafe { CreateIconIndirect(&icon_info) };

    // Clean up temporary GDI bitmaps and memory DC (CreateIconIndirect copies the bitmap internally).
    // SAFETY: DeleteObject and DeleteDC safely release GDI resources.
    unsafe {
        let _ = DeleteObject(hbm_color.into());
        let _ = DeleteObject(hbm_mask.into());
        let _ = DeleteDC(mem_dc);
    }

    hicon_res
}

/// Safely destroys an owned `HICON`.
pub fn destroy_hicon(hicon: HICON) {
    if !hicon.is_invalid() {
        // SAFETY: DestroyIcon frees the HICON handle.
        unsafe {
            let _ = DestroyIcon(hicon);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use magicbridge_core::Config;

    #[test]
    fn test_canvas_initialization_and_pixel_ops() {
        let mut canvas = RgbaCanvas::new(16, 16);
        assert_eq!(canvas.pixels.len(), 256);
        assert_eq!(canvas.get_pixel(0, 0), 0);

        canvas.set_pixel(5, 5, 0xFF112233);
        assert_eq!(canvas.get_pixel(5, 5), 0xFF112233);

        canvas.fill_rect(2, 2, 4, 4, 0xFFAABBCC);
        assert_eq!(canvas.get_pixel(2, 2), 0xFFAABBCC);
        assert_eq!(canvas.get_pixel(4, 4), 0xFFAABBCC);
        assert_eq!(canvas.get_pixel(5, 5), 0xFF112233);
    }

    #[test]
    fn test_render_canvas_disconnected() {
        let mut state = DaemonState::new(Config::default(), None);
        state.is_connected = false;

        let canvas = render_canvas(16, 16, &state);
        assert_eq!(canvas.width, 16);
        assert_eq!(canvas.height, 16);
        // Should contain non-zero pixels for keyboard frame
        assert!(canvas.pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_render_canvas_connected_battery_levels() {
        let mut state = DaemonState::new(Config::default(), None);
        state.is_connected = true;

        for pct in [0, 10, 15, 50, 85, 100] {
            state.battery_percent = Some(pct);
            state.is_charging = false;
            let canvas = render_canvas(16, 16, &state);
            assert!(canvas.pixels.iter().any(|&p| p != 0));

            // Charging state
            state.is_charging = true;
            let canvas_charging = render_canvas(16, 16, &state);
            assert!(canvas_charging.pixels.iter().any(|&p| p != 0));
        }
    }

    #[test]
    fn test_render_canvas_paused_and_game_mode() {
        let mut state = DaemonState::new(Config::default(), None);
        state.is_connected = true;
        state.battery_percent = Some(80);

        state.enabled = false;
        let canvas_paused = render_canvas(16, 16, &state);
        assert!(canvas_paused.pixels.iter().any(|&p| p != 0));

        state.enabled = true;
        state.game_mode_active = true;
        let canvas_game_mode = render_canvas(16, 16, &state);
        assert!(canvas_game_mode.pixels.iter().any(|&p| p != 0));
    }

    #[test]
    fn test_create_battery_hicon_win32() {
        let mut state = DaemonState::new(Config::default(), None);
        state.is_connected = true;
        state.battery_percent = Some(75);

        let hicon_res = create_battery_tray_icon(&state);
        assert!(hicon_res.is_ok());
        let hicon = hicon_res.unwrap();
        assert!(!hicon.is_invalid());
        destroy_hicon(hicon);
    }
}
