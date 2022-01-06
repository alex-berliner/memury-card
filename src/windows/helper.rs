use std::path::{Path, PathBuf};
use std::io;
use winreg::enums::*;
use winreg::RegKey;
use std::ffi::OsString;

static current_version:&'static str = r"Software\Microsoft\Windows\CurrentVersion\Run";
static startup_approved:&'static str = r"software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
static regname:&'static str = "MemuryCard";

pub fn install(p: PathBuf, enabled: bool) -> io::Result<()> {
    // for some reason using get_subkey doesn't work, only create_subkey
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let startup = Path::new(current_version);
    let (key, _) = hkcu.create_subkey(&startup).unwrap();
    match key.set_value(&regname, &p.to_str().unwrap()) {
        Ok(e) => (),
        _ => println!("Could not set CurrentVersion"),
    };

    let startup_enabled = Path::new(startup_approved);
    let (key, _) = hkcu.create_subkey(&startup_enabled).unwrap();
    let mut data = match key.get_raw_value(&regname) {
        Ok(e) => e,
        _ => {
            let mut bytes: Vec<u8> = vec![0,0,0,0,0,0,0,0,0,0,0,0];
            winreg::RegValue{ vtype: REG_BINARY, bytes: bytes }
        }
    };
    data.bytes[0] &= 0xFC;
    data.bytes[0] |= if enabled {2} else {3};
    match key.set_raw_value(&regname, &data) {
        Ok(e) => (),
        _ => println!("Could not set StartupApproved"),
    };
    Ok(())
}

pub fn uninstall() {
    // for some reason using get_subkey doesn't work, only create_subkey
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let startup = Path::new(current_version);
    let (key, _) = hkcu.create_subkey(current_version).expect("Could not open current_version");
    // TODO: add logger support and add an info print for the entries not existing
    key.delete_value(&regname);

    let startup_enabled = Path::new(startup_approved);
    let (key, _) = hkcu.create_subkey(startup_enabled).expect("Could not open startup_enabled");
    // TODO: add logger support and add an info print for the entries not existing
    key.delete_value(&regname);
}
