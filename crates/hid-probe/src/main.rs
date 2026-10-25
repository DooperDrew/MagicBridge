//! Milestone 0 diagnostic CLI (developer tool, console output, not localised).
//!
//! Skeleton only (M0.1). Device enumeration arrives in M0.2, live dumping in M0.3.

fn main() {
    println!(
        "hid-probe {}: skeleton, no probes implemented yet (target VID 0x{:04X}).",
        env!("CARGO_PKG_VERSION"),
        magicbridge_core::APPLE_VID
    );
}
