//! Milestone 0 diagnostic CLI for Apple Magic Keyboard (A1843).
//!
//! Subcommands:
//! - `list` (default): enumerate Apple HID collections, report descriptors, and Raw Input devices (M0.2).
//! - `dump`: live-dump Raw Input, direct HID reads, and `WH_KEYBOARD_LL` side by side with pairing (M0.3, M0.6).
//! - `battery`: probe HID feature/input reports and PnP/Bluetooth battery properties (M0.5).
//!
//! Flags:
//! - `--all`: include non-Apple HID devices.
//! - `--log <path>`: tee all output to `<path>` (flushed every line).
//! - `--no-hook`: disable `WH_KEYBOARD_LL` in `dump`.
//! - `--no-hid-read`: disable direct `ReadFile` threads in `dump`.

mod battery;
mod dump;
mod hid;
mod list;
mod out;
mod rawinput;

fn main() {
    out::start_clock();

    let mut args = std::env::args().skip(1).peekable();
    let mut cmd = String::from("list");
    let mut all = false;
    let mut hook = true;
    let mut hid_read = true;
    let mut log_path: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "list" | "dump" | "battery" => cmd = arg,
            "--all" => all = true,
            "--no-hook" => hook = false,
            "--no-hid-read" => hid_read = false,
            "--log" => {
                if let Some(p) = args.next() {
                    log_path = Some(p);
                } else {
                    eprintln!("--log requires a file path");
                    std::process::exit(2);
                }
            }
            "-h" | "--help" | "help" => {
                print_help();
                return;
            }
            other => {
                eprintln!("unknown argument: {other}");
                print_help();
                std::process::exit(2);
            }
        }
    }

    if let Some(ref path) = log_path
        && let Err(e) = out::set_log_file(path)
    {
        eprintln!("failed to open log file {path}: {e}");
        std::process::exit(1);
    }

    outln!(
        "hid-probe {} — cmd={cmd} all={all} log={}",
        env!("CARGO_PKG_VERSION"),
        log_path.as_deref().unwrap_or("none")
    );

    match cmd.as_str() {
        "list" => list::run(all),
        "battery" => battery::run(all),
        "dump" => {
            if let Err(e) = dump::run(&dump::Options {
                all,
                hook,
                hid_read,
            }) {
                outln!("dump failed: {e}");
                std::process::exit(1);
            }
        }
        _ => unreachable!(),
    }
}

fn print_help() {
    println!(
        "\
Usage: hid-probe [list|dump|battery] [options]

Commands:
  list       List Apple (0x05AC) HID collections, report descriptors & Raw Input devices (default)
  dump       Live-dump Raw Input, direct HID reports, and WH_KEYBOARD_LL with pairing
  battery    Probe HID feature/input reports and PnP/Bluetooth battery properties

Options:
  --all           Include non-Apple HID devices
  --log <path>    Also write all output to <path>
  --no-hook       (dump) Do not install WH_KEYBOARD_LL hook
  --no-hid-read   (dump) Do not spawn direct ReadFile threads on HID collections"
    );
}
