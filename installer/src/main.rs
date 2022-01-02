#[cfg(windows)]
use std::ffi::OsString;
use windows_service::{
    service::{ServiceAccess, ServiceErrorControl, ServiceInfo, ServiceStartType, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

fn install() -> windows_service::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_binary_path = ::std::env::current_exe().unwrap().with_file_name("memury_card.exe");

    let service_info = ServiceInfo {
        name: OsString::from("memury_card"),
        display_name: OsString::from("Memury Card"),
        service_type: ServiceType::OWN_PROCESS,
        // AutoStart OnDemand Disabled
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };
    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description("Emulator save manager")?;
    Ok(())
}

fn uninstall() -> windows_service::Result<()> {
    use std::{thread, time::Duration};
    use windows_service::{
        service::{ServiceAccess, ServiceState},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service("memury_card", service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        service.stop()?;
        // Wait for service to stop
        thread::sleep(Duration::from_secs(1));
    }

    service.delete()?;
    Ok(())
}

use std::io::prelude::*;
use std::fs::File;
use argh::FromArgs;

#[derive(FromArgs)]
/// Reach new heights.
struct Installer {
    /// whether or not to install
    #[argh(switch, short = 'i')]
    install: bool,

    // /// how high to go
    // #[argh(option)]
    // height: usize,

    // /// an optional nickname for the pilot
    // #[argh(option)]
    // pilot_nickname: Option<String>,
}

fn main() -> windows_service::Result<()> {
    let up: Installer = argh::from_env();
    if up.install {
        install();
    } else {
        uninstall();
    }
    Ok(())
}

#[cfg(not(windows))]
fn main() {
    panic!("This program is only intended to run on Windows.");
}
