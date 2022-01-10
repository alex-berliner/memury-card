use std::process::Command;
use std::path::Path;
use winreg::enums::*;
use winreg::RegKey;

static CURRENT_VERSION:&'static str = r"Software\Microsoft\Windows\CurrentVersion\Run";
static STARTUP_APPROVED:&'static str = r"software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
static REGNAME:&'static str = "MemuryCard";

pub fn install(enabled: bool) {
    // TODO: for some reason using get_subkey doesn't work, only create_subkey, fix it when I have more time :)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let startup = Path::new(CURRENT_VERSION);
    let (key, _) = hkcu.create_subkey(&startup).unwrap();
    let mut startup_val = std::env::current_exe().unwrap().to_str().unwrap().to_string();
    startup_val.push_str(" -b");
    match key.set_value(&REGNAME, &startup_val) {
        Ok(_) => (),
        _ => log::info!("Could not set CurrentVersion"),
    };

    let backbat = std::env::current_dir().unwrap();
    log::info!("{:?}", backbat);

    let startup_enabled = Path::new(STARTUP_APPROVED);
    let (key, _) = hkcu.create_subkey(&startup_enabled).unwrap();
    let mut data = match key.get_raw_value(&REGNAME) {
        Ok(e) => e,
        _ => {
            let bytes: Vec<u8> = vec![0; 12];
            winreg::RegValue{ vtype: REG_BINARY, bytes: bytes }
        }
    };
    data.bytes[0] &= 0xFC;
    data.bytes[0] |= if enabled {2} else {3};
    match key.set_raw_value(&REGNAME, &data) {
        Ok(_) => (),
        _ => log::info!("Could not set StartupApproved"),
    };
}

pub fn uninstall() {
    // TODO: for some reason using get_subkey doesn't work, only create_subkey, fix it when I have more time :)
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let (key, _) = hkcu.create_subkey(CURRENT_VERSION).expect("Could not open CURRENT_VERSION");
    // TODO: add logger support and add an info print for the entries not existing
    key.delete_value(&REGNAME).expect("Could not delete registry value");

    let startup_enabled = Path::new(STARTUP_APPROVED);
    let (key, _) = hkcu.create_subkey(startup_enabled).expect("Could not open startup_enabled");
    // TODO: add logger support and add an info print for the entries not existing
    key.delete_value(&REGNAME).expect("Could not delete registry value");
}

pub fn send_to_background() {
    let mut cmd = "".to_string();
    cmd.push_str(&"powershell -WindowStyle Hidden -Command \"");
    cmd.push_str(std::env::current_exe().unwrap().to_str().unwrap());
    cmd.push_str(&"\"\n");
    let _status = Command::new("powershell").arg(&cmd).spawn().expect("failed to execute process");
}
