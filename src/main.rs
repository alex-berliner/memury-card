mod service;
mod helper;
mod windows;
use chrono;
use argh::FromArgs;
use std::{thread, time};
use std::io::Write;
use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Root};
use log4rs::append::console::ConsoleAppender;

#[derive(FromArgs)]
/// Memury Card cli args
struct MCArgs {
    /// add program to startup
    #[argh(switch, short = 'i')]
    install: bool,

    /// remove program from startup
    #[argh(switch, short = 'u')]
    uninstall: bool,

    /// launch as background process
    #[argh(switch, short = 'b')]
    background: bool,
}

fn main() {
    // set cwd to path of exe
    let cwd = std::env::current_dir().unwrap();
    let mut exedir = std::env::current_exe().unwrap();
    exedir.pop();
    std::env::set_current_dir(&exedir);
    let cwd = std::env::current_dir().unwrap();

    // set up logging
    let dt = chrono::Utc::now();
    let timestamp: i64 = dt.timestamp();
    let log = format!(r"{}/scary/log/log{}.log", exedir.to_str().unwrap(), timestamp);
    println!("log: {}", log);
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(&log).unwrap();

    let stdout = ConsoleAppender::builder().build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder()
                   .appender("stdout")
                   .appender("logfile")
                   .build(LevelFilter::Info)).unwrap();

    log4rs::init_config(config).unwrap();

    // log start time
    log::info!("{}", chrono::offset::Local::now());

    // parse args
    let mcargs: MCArgs = argh::from_env();
    if mcargs.uninstall {
        log::info!("mcargs.uninstall");
        windows::helper::uninstall();
    } else if mcargs.install {
        log::info!("mcargs.install");
        windows::helper::install(true);
    } else if mcargs.background {
        log::info!("mcargs.background");
        windows::helper::send_to_background();
        std::process::exit(0);
    } else {
        log::info!("service::service::run()");
        service::service::run();
    }
    log::info!("exit");
}
