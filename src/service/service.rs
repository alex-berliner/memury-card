use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::WalkDir;


#[derive(StructOpt)]
struct Cli {
    #[structopt(default_value = "settings.json")]
    settings: std::path::PathBuf,
}

enum FileOpCmd {
    Watch(SaveDef),
    Unwatch(String),
    Copy(PathBuf),
    Scan(),
}

struct SaveFile {
}

enum RuleList {
    Allowed(Vec<String>),
    Disallowed(Vec<String>),
}

struct SaveDir {
    rule_list: RuleList,
    files: HashSet<PathBuf>,
}

enum SaveOpts {
    File(SaveFile),
    Dir(SaveDir),
}

struct SaveDef {
    name: String,
    path: PathBuf,
    sync_loc: PathBuf,
    options: SaveOpts,
}

impl SaveDef {
    fn print(&self) {
        log::info!("{:?}", self.path);
    }
}

impl SaveDir {
    #[allow(dead_code)]
    fn print(&self) {
        let rule_list = match &self.rule_list {
            RuleList::Allowed(v) =>  {
                log::info!("allowed");
                v
            }
            RuleList::Disallowed(v) => {
                log::info!("disallowed");
                v
            }
        };
        for i in 0 .. rule_list.len() {
            log::info!("{}", rule_list[i]);
        }
    }


    fn meets_rules(&self, p: &PathBuf) -> bool {
        match &self.rule_list {
            RuleList::Allowed(v) =>  {
                let ext = p.extension().unwrap().to_str().unwrap();
                for ftype in v {
                    if ftype == ext {
                        return false;
                    }
                }
                true
            }
            RuleList::Disallowed(v) => {
                let pstr = p.to_str().unwrap();
                log::info!("parsing disallowed as {}", pstr);
                for disallowed in v {
                    log::info!("disallowed {} pstr.ends_with(disallowed) {}", disallowed, pstr.ends_with(disallowed));
                    if pstr.ends_with(disallowed) {
                        return false;
                    }
                }
                true
            }
        }
    }
}

// cli thread
fn interactive(json_dir: &str, file_op_tx: &mpsc::Sender<FileOpCmd>) {
    find_json_settings(json_dir, file_op_tx);
    file_op_tx.send(FileOpCmd::Scan());
    loop {
        log::info!("Enter command: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        match input {
            "s" => {
                file_op_tx.send(FileOpCmd::Scan());
            }
            _ => (),
        }
    }
}

// thread to handle events coming in on the file watcher
fn save_scanner(
    json_dir: &str,
    file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
    file_op_tx: &mpsc::Sender<FileOpCmd>,
) {
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) | DebouncedEvent::Create(p) => {
                    let p = PathBuf::from(p);
                    log::info!("{:?}", p);
                    file_op_tx.send(FileOpCmd::Copy(p));
                }
                DebouncedEvent::NoticeWrite(p) => {
                    // log::info!("NoticeWrite {:?}", p);
                }
                DebouncedEvent::Remove(p) => {
                    log::info!("Remove {:?}", p);
                }
                DebouncedEvent::NoticeRemove(p) => {
                    log::info!("NoticeRemove {:?}", p);
                }
                DebouncedEvent::Rename(a, b) => {
                    log::info!("Rename {:?} -> {:?}", a, b);
                }
                _ => (),
            },
            Err(e) => log::info!("watch error: {:?}", e),
        };
    }
}

// look for the path as registered in the save_map. both files and directories can be registered so if it's a directory
// we need to chop off portions of the file path until we either find the path that the file was registered under or
// get to the root (ie bad file). files under paths aren't registered, only find events when the dirs has an event
fn find_appropriate_savedef_path(p: &PathBuf, save_map: &HashMap<PathBuf, SaveDef>) -> Result<PathBuf, String> {
    let mut p = p.clone();

    while !save_map.contains_key(&p) && p.parent() != None {
        p.pop();
    }

    if !save_map.contains_key(&p) {
        return Err("Path not in save map".to_string());
    }

    Ok(p)
}

// thread function for file io heavy lifting
fn save_watcher(
    sync_dir: &str,
    file_scan_tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
    file_op_tx: std::sync::mpsc::Sender<FileOpCmd>,
    file_op_rx: std::sync::mpsc::Receiver<FileOpCmd>,
) {
    let mut watcher = watcher(file_scan_tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<PathBuf, SaveDef> = HashMap::new();
    loop {
        match file_op_rx.recv().unwrap() {
            FileOpCmd::Watch(save) => {
                print!("watch ");
                save.print();
                let p = save.path.clone();
                if !p.exists() {
                    log::info!("{:?} doesn't exist", p);
                }
                let err = watcher.watch(&p, RecursiveMode::Recursive);
                save_map.entry(p.clone()).or_insert(save);
                // TODO: if err...
            }
            FileOpCmd::Unwatch(rmpath) => {
                // watcher.unwatch(&entry).unwrap();
            }
            FileOpCmd::Copy(src) => {
                let key = find_appropriate_savedef_path(&src, &save_map).unwrap();
                let err = format!("could not find {:?}", key);
                let save_reg = save_map.get(&key).expect(&err);
                let mut sync_loc = PathBuf::from(save_reg.sync_loc.clone());
                let has_appropriate_type = match &save_reg.options {
                    SaveOpts::Dir(e) => { e.meets_rules(&src) },
                    _ => true,
                };


                if has_appropriate_type {
                    let mut dst = PathBuf::from(sync_dir);
                    let src_file_name = src.file_name().expect("this was probably not a file");

                    dst.push(sync_loc);
                    dst.set_extension("");
                    std::fs::create_dir_all(&dst);

                    dst.push(&src_file_name);
                    dst.set_extension(src.extension().unwrap());
                    log::info!("copy {:?} into save manager at {:?}", src, dst);
                    match std::fs::copy(&src, &dst) {
                        Err(e) => {
                            log::info!("\nfile copy error: {:?} {:?} {:?}", e, src, dst);
                            log::info!("{:?} exists: {:?}", src, src.exists());
                            log::info!("{:?} exists: {:?}\n", dst, dst.exists());
                            ()
                        },
                        _ => (),
                    }
                }
            }
            FileOpCmd::Scan() => {
                for (key, value) in &save_map {
                    for entry in WalkDir::new(key).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                        let p = PathBuf::from(entry.path());
                        if p.is_file() {
                            file_op_tx.send(FileOpCmd::Copy(PathBuf::from(entry.path())));
                        }
                    }
                }
            }
        }
    }
}

// parse user generated json files indicating location of content storage areas
fn parse_save_json(json_file: &str, save_accu: &mut Vec<SaveDef>) {
    let bytes = std::fs::read_to_string(json_file).unwrap();
    let json: Value = serde_json::from_str(&bytes).unwrap();
    let saves = json["saves"].as_array().unwrap();

    for i in 0..saves.len() {
        // json elements with the "dir" field populated are directories
        let mut path = PathBuf::new();
        let is_dir = saves[i]["dir"] != Value::Null;
        let name = crate::helper::strip_quotes(saves[i]["name"].as_str().unwrap());
        let sync_loc = PathBuf::from(crate::helper::strip_quotes(saves[i]["sync_loc"].as_str().unwrap()));

        let mut saveopt = if is_dir {
            path.push(crate::helper::strip_quotes(saves[i]["dir"].as_str().unwrap()));
            log::debug!("{:?}", path);
            if saves[i]["allowed"] != Value::Null && saves[i]["disallowed"] != Value::Null {
                log::error!("{:?} can only have an allow list or disallow list", path);
                continue;
            }

            if saves[i]["allowed"] == Value::Null && saves[i]["disallowed"] == Value::Null {
                log::error!("{:?} must have either allow or disallow list", path);
                continue;
            }

            let rule_list = if saves[i]["allowed"] != Value::Null {
                let allowed = saves[i]["allowed"].as_array().unwrap();
                let mut allowed_vec: Vec<String> = vec![];
                for j in 0..allowed.len() {
                    let filetypes_str = allowed[j].as_str().unwrap().to_string();
                    if filetypes_str.len() > 0 {
                        allowed_vec.push(filetypes_str);
                    }
                }
                RuleList::Allowed({allowed_vec})
            } else /* if saves[i]["disallowed"] != Value::Null */ {
                let disallowed = saves[i]["disallowed"].as_array().unwrap();
                let mut disallowed_vec: Vec<String> = vec![];
                for j in 0..disallowed.len() {
                    let disallowed_str = disallowed[j].as_str().unwrap().to_string();
                    if disallowed_str.len() > 0 {
                        disallowed_vec.push(disallowed_str);
                    }
                }
                RuleList::Disallowed({disallowed_vec})
            };
            let mut savedir = SaveDir {
                rule_list: rule_list,
                files: HashSet::new(),
            };
            SaveOpts::Dir(savedir)
        } else {
            path.push(crate::helper::strip_quotes(saves[i]["file"].as_str().unwrap()));
            let savefile = SaveFile {
            };
            SaveOpts::File(savefile)
        };
        let savedef = SaveDef {
            name: name,
            path: path,
            sync_loc: sync_loc,
            options: saveopt,
        };
        save_accu.push(savedef);
    }
}

// find all files in @json_dir that end in .json, return a vector of SaveDef's from them
fn get_json_settings_descriptors(json_dir: &str) -> Vec<SaveDef> {
    let json_dir = crate::helper::strip_quotes(json_dir);
    let mut save_accu: Vec<SaveDef> = vec![];

    for entry in WalkDir::new(json_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();

        if f_name.ends_with(".json") {
            let entry = entry.path().to_str().unwrap();
            parse_save_json(&entry, &mut save_accu);
        }
    }
    save_accu
}

// crawl through saves listed from save files and send results to watcher thread
fn find_json_settings(json_dir: &str, file_op_tx: &mpsc::Sender<FileOpCmd>) {
    let saves = get_json_settings_descriptors(json_dir);
    for e in saves {
        file_op_tx.send(FileOpCmd::Watch(e));
    }
}

pub fn run() {
    let args = Cli::from_args();
    let parse = crate::helper::parse_json(&args.settings).unwrap();
    let tracker_dir1 = parse["tracker_dir"].to_string();
    let tracker_dir2 = tracker_dir1.clone();

    let sync_dir = crate::helper::strip_quotes(&parse["sync_dir"].to_string());

    let (file_scan_tx, file_scan_rx) = mpsc::channel();
    let (file_op_tx, file_op_rx) = mpsc::channel();
    let file_op_tx2 = file_op_tx.clone();
    let file_op_tx3 = file_op_tx.clone();

    let save_scanner_handle = thread::spawn(move || {
        save_scanner(&tracker_dir1, file_scan_rx, &file_op_tx);
    });
    let save_watcher_handle = thread::spawn(move || {
        save_watcher(&sync_dir, file_scan_tx, file_op_tx3, file_op_rx);
    });
    let interactive_handle = thread::spawn(move || {
        interactive(&tracker_dir2, &file_op_tx2);
    });

    save_scanner_handle.join().unwrap();
    save_watcher_handle.join().unwrap();
    interactive_handle.join().unwrap();
}
