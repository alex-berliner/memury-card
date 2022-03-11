#[cfg(target_os = "linux")]
pub fn install(enabled: bool) {
    crate::linux::helper::install(enabled);
}

#[cfg(target_os = "linux")]
pub fn uninstall() {
    crate::linux::helper::uninstall();
}

#[cfg(target_os = "linux")]
pub fn send_to_background() {
    crate::linux::helper::send_to_background();
}

#[cfg(target_os = "windows")]
pub fn install(enabled: bool) {
    crate::windows::helper::install(enabled);
}

#[cfg(target_os = "windows")]
pub fn uninstall() {
    crate::windows::helper::uninstall();
}

#[cfg(target_os = "windows")]
pub fn send_to_background() {
    crate::windows::helper::send_to_background();
}

pub fn enable() { }

pub fn disable() { }
