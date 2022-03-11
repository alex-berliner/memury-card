mod helper;
mod linux;
mod service;
mod windows;
use argh::FromArgs;
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use std::path::PathBuf;

#[derive(FromArgs)]
/// Memury Card cli args
struct MCArgs {
    /// path to main settings file
    #[argh(option, short = 's', default = "PathBuf::from(\"settings.json\")")]
    settings: PathBuf,

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
    helper::print_splash();
    let mut exedir = std::env::current_exe().unwrap();
    exedir.pop();
    std::env::set_current_dir(&exedir).unwrap();

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
        service::system::uninstall();
    } else if mcargs.install {
        log::info!("mcargs.install");
        service::system::install(true);
    } else if mcargs.background {
        log::info!("mcargs.background");
        service::system::send_to_background();
        std::process::exit(0);
    } else {
        log::info!("service::service::run()");
        service::service::run(&mcargs.settings);
    }
    log::info!("exit");
}
