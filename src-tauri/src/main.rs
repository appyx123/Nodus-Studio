#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> Result<(), nodus_studio_lib::NodusError> {
    nodus_studio_lib::run()
}
