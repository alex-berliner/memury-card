/*
watcher can watch paths and files but the events only carry the exact file that was modified, not the path that registered
the file. this makes it hard to know which rules to apply to the file, particularly with path-watched files because the
files don't have an explicit reference to do something like reference into a hashmap with. the path registered for a
file could be determined by repeatedly chopping off the last part of the file and checking for it in the hash map.
This would solve everything but two overlapping directories being registered, which seems like a reasonable restriction
to the user and something that might be easy to validate at startup.

single SaveDir with contextual settings in json, figure out fields on the fly
hashmap stores

resolving same-name files when registering directories
register /a

/a/b/s.sav
/a/c/s.sav

"preserve_structure": "true" (default: true)

TDD

*/

use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use serde_json::{Result, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::WalkDir;

mod helper;


impl SaveDef {
    fn print(&self) {
        println!("{:?}", self.path);
    }
}

impl SaveDir {
    #[allow(dead_code)]
    fn print(&self) {
        println!("dir:      {:?}", self.dir);
        for j in 0..self.filetypes.len() {
            println!("filetype: {}", self.filetypes[j]);
        }
    }
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(default_value = "settings.json")]
    settings: std::path::PathBuf,
}

struct Savedata {
    // filemap: HashMap<String, String>,
    saveloc: SaveDir,
}

enum HashmapCmd {
    Watch(SaveDef),
    Unwatch(String),
    Copy(PathBuf),
}

fn interactive(json_dir: &str, file_add_tx: &mpsc::Sender<HashmapCmd>) {
    loop {
        println!("Enter command: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        match input {
            "s" => {
                find_json_settings(json_dir, file_add_tx);
            }
            _ => (),
        }
    }
}

struct SaveFile {
    file: PathBuf,
    sync_loc: PathBuf,
}

struct SaveDir {
    dir: PathBuf,
    filetypes: Vec<String>,
    files: HashSet<PathBuf>,
}

enum SaveOpts {
    File(SaveFile),
    Dir(SaveDir),
    DirFile(),
}

struct SaveDef {
    path: PathBuf,
    options: SaveOpts,
}

fn save_scanner(
    json_dir: &str,
    file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
    file_add_tx: &mpsc::Sender<HashmapCmd>,
) -> Result<()> {
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    let p = PathBuf::from(p);
                    println!("{:?}", p);
                    file_add_tx.send(HashmapCmd::Copy(p));
                }
                DebouncedEvent::NoticeWrite(p) => {
                    println!("NoticeWrite {:?}", p);
                }
                // update watch list with newly created file
                DebouncedEvent::Create(p) => {
                    println!("Create {:?}", p);
                    let towatch = SaveDef {
                        path: p,
                        options: SaveOpts::DirFile(),
                    };
                    file_add_tx.send(HashmapCmd::Watch(towatch));
                }
                DebouncedEvent::Remove(p) => {
                    println!("Remove {:?}", p);
                }
                DebouncedEvent::NoticeRemove(p) => {
                    println!("NoticeRemove {:?}", p);
                }
                DebouncedEvent::Rename(a, b) => {
                    println!("Rename {:?} -> {:?}", a, b);
                }
                _ => (),
            },
            Err(e) => println!("watch error: {:?}", e),
        };
    }
}

fn find_appropriate_savedef(p: &PathBuf, save_map: &HashMap<PathBuf, SaveDef>) -> SaveDef {
    println!("find_appropriate_savedef");
    let mut p = p.clone();
    // save_map.get()
    let ret = SaveDef {
        path: PathBuf::from(""),
        options: SaveOpts::File(SaveFile {
            file: PathBuf::from(""),
            sync_loc: PathBuf::from(""),
        })
    };

    if !save_map.contains_key(&p) {
        p.pop();
        println!("{:?}", p);
    } else {
        println!("it contains");
    }

    ret
}

fn save_watcher(
    sync_dir: &str,
    file_scan_tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
    file_add_rx: std::sync::mpsc::Receiver<HashmapCmd>,
) {
    let mut watcher = watcher(file_scan_tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<PathBuf, SaveDef> = HashMap::new();
    loop {
        match file_add_rx.recv().unwrap() {
            HashmapCmd::Watch(save) => {
                print!("watch ");
                save.print();
                let p = save.path.clone();
                let err = watcher.watch(&p, RecursiveMode::Recursive);
                save_map.entry(p.clone()).or_insert(save);
                // TOOD: if err...
                // println!("{:?}", save_map.contains_key(&save.path));
            }
            HashmapCmd::Unwatch(rmpath) => {
                // watcher.unwatch(&entry).unwrap();
            }
            HashmapCmd::Copy(src) => {
                // dont copy right away, poll from hashmap to get settings to see whehter to append something, etc
                let x = find_appropriate_savedef(&src, &save_map);
                let mut dst = PathBuf::from(sync_dir);
                let src_file_name = src.file_name().expect("this was probably not a file");
                dst.push(&src_file_name);
                dst.set_extension(src.extension().unwrap());
                println!("copy {:?} into save manager at {:?}", src, dst);
                std::fs::create_dir_all(&dst);
                std::fs::copy(&src, &dst);
            }
        }
    }
}

fn parse_save_json(json_file: &str, save_accu: &mut Vec<SaveDef>) {
    let bytes = std::fs::read_to_string(json_file).unwrap();
    let json: Value = serde_json::from_str(&bytes).unwrap();
    let saves = json["saves"].as_array().unwrap();

    for i in 0..saves.len() {
        // json elements with the "dir" field populated are directories
        let mut path = PathBuf::new();
        let is_dir = saves[i]["dir"] != Value::Null;
        let mut saveopt = if is_dir {
            path.push(helper::strip_quotes(saves[i]["dir"].as_str().unwrap()));
            let mut savedir = SaveDir {
                dir: PathBuf::from("/"), //TODO rm?
                filetypes: vec![],
                files: HashSet::new(),
            };
            let mut filetypes = saves[i]["filetypes"].as_array().unwrap();
            for j in 0..filetypes.len() {
                savedir.filetypes.push(filetypes[j].as_str().unwrap().to_string());
            }
            SaveOpts::Dir(savedir)
        } else {
            path.push(helper::strip_quotes(saves[i]["file"].as_str().unwrap()));
            let savefile = SaveFile {
                file: PathBuf::from("/x.txt"),
                sync_loc: PathBuf::from("asdasdas"),
            };
            SaveOpts::File(savefile)
        };
        let savedef = SaveDef {
            path: path,
            options: saveopt,
        };
        save_accu.push(savedef);
    }
}

// find all files in @json_dir that end in .json, return a vector of SaveDef's from them
fn get_json_settings_descriptors(json_dir: &str) -> Vec<SaveDef> {
    let json_dir = helper::strip_quotes(json_dir);
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
            // file_add_tx.send(entry.clone().to_string());
        }
    }
    save_accu
}

fn get_save_watch_entries(savelocs: &Vec<SaveDir>) -> Vec<String> {
    let mut save_watch_entries: Vec<String> = vec![];
    // find all save files in each directory
    // add things to the hash map based on the settings from the savelocs
    for e in savelocs.iter() {
        for entry in WalkDir::new(&e.dir).follow_links(true).into_iter().filter_map(|e| e.ok())
        {
            let type_satisfy = false;
            for ftype in &e.filetypes {
                let f_name = entry.file_name().to_string_lossy();
                if f_name.ends_with(ftype) {
                    // println!("{}: {}", ftype, entry.path().to_str().unwrap());
                    save_watch_entries.push(entry.path().to_str().unwrap().to_string());
                }
            }
        }
    }
    save_watch_entries
}

fn get_dir_path_files(saves: &Vec<SaveDef>) -> Vec<SaveDef> {
    let mut retsaves: Vec<SaveDef> = vec![];
    for e in saves {
        if matches!(e.options, SaveOpts::Dir(_)) {
            // e.print();
            for entry in WalkDir::new(&e.path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                // println!("zzzzzzzzzzz {:?}", entry);
                let type_satisfy = false;
                match &e.options {
                    SaveOpts::DirFile() => { /* println!("DirFile") */ }
                    SaveOpts::File(ee)  => { /* println!("File") */ }
                    SaveOpts::Dir(ee) => {
                        // println!("ee");
                        for ftype in &ee.filetypes {
                            let f_name = entry.file_name().to_string_lossy();
                            if f_name.ends_with(ftype) {
                                println!("{:?} {}: {}", e.path, ftype, entry.path().to_str().unwrap());
                                // save_watch_entries.push(entry.path().to_str().unwrap().to_string());
                                // println!("{:?}", f_name);
                                retsaves.push(SaveDef {
                                    path: entry.path().to_path_buf(),
                                    options: SaveOpts::DirFile(),
                                });
                            }
                        }

                    }
                }
            }
        }
    }

    retsaves
}

// crawl through saves listed from save files and send results to watcher thread
fn find_json_settings(json_dir: &str, file_add_tx: &mpsc::Sender<HashmapCmd>) {
    let saves = get_json_settings_descriptors(json_dir);

    let dir_saves = get_dir_path_files(&saves);

    for e in saves {
        file_add_tx.send(HashmapCmd::Watch(e));
    }
    for e in dir_saves {
        file_add_tx.send(HashmapCmd::Watch(e));
    }
}

fn main() {
    let args = Cli::from_args();
    let parse = helper::parse_json(&args.settings).unwrap();
    let tracker_dir1 = parse["tracker_dir"].to_string();
    let tracker_dir2 = tracker_dir1.clone();

    let sync_dir = helper::strip_quotes(&parse["sync_dir"].to_string());

    let (file_scan_tx, file_scan_rx) = mpsc::channel();
    let (file_add_tx, file_add_rx) = mpsc::channel();
    let file_add_tx2 = file_add_tx.clone();

    let save_scanner_handle = thread::spawn(move || {
        save_scanner(&tracker_dir1, file_scan_rx, &file_add_tx);
    });
    let save_watcher_handle = thread::spawn(move || {
        save_watcher(&sync_dir, file_scan_tx, file_add_rx);
    });
    let interactive_handle = thread::spawn(move || {
        interactive(&tracker_dir2, &file_add_tx2);
    });

    save_scanner_handle.join().unwrap();
    save_watcher_handle.join().unwrap();
    interactive_handle.join().unwrap();
}
