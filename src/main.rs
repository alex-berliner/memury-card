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

*/

use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use serde_json::{Result, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::WalkDir;

mod helper;

struct SaveDir {
    dir: PathBuf,
    filetypes: Vec<String>,
}

struct SaveFile {
    file: PathBuf,
    sync_loc: PathBuf,
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
    WatchDir(SaveDir),
    WatchFile(SaveFile),
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
                find_saves(json_dir, file_add_tx);
            }
            _ => (),
        }
    }
}

fn parse_save_json(json_file: &str, dir_accu: &mut Vec<SaveDir>, file_accu: &mut Vec<SaveFile>) {
    let bytes = std::fs::read_to_string(json_file).unwrap();
    let json: Value = serde_json::from_str(&bytes).unwrap();
    let saves = json["saves"].as_array().unwrap();

    for i in 0..saves.len() {
        // json elements with the "dir" field populated are directories
        if saves[i]["dir"] != Value::Null {
            let mut save = SaveDir {
                dir: PathBuf::from(helper::strip_quotes(saves[i]["dir"].as_str().unwrap())),
                filetypes: vec![],
            };
            let filetypes = saves[i]["filetypes"].as_array().unwrap();
            for j in 0..filetypes.len() {
                save.filetypes
                    .push(filetypes[j].as_str().unwrap().to_string());
            }
            dir_accu.push(save);
        } else if saves[i]["file"] != Value::Null {
            let save = SaveFile {
                file: PathBuf::from(helper::strip_quotes(saves[i]["file"].as_str().unwrap())),
                sync_loc: if saves[i]["sync_loc"] != Value::Null {
                    PathBuf::from(helper::strip_quotes(saves[i]["sync_loc"].as_str().unwrap()))
                } else {
                    PathBuf::from("")
                },
            };
            file_accu.push(save);
        }
    }
}

fn save_scanner(
    json_dir: &str,
    file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
    file_add_tx: &mpsc::Sender<HashmapCmd>,
) -> Result<()> {
    // find_saves(json_dir, file_add_tx);
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    let p = PathBuf::from(p);
                    println!("{:?}", p);
                    file_add_tx.send(HashmapCmd::Copy(p));
                }
                DebouncedEvent::NoticeWrite(p) => {
                    println!("NoticeWrite {:?}", p)
                }
                DebouncedEvent::Create(p) => {
                    println!("Create {:?}", p)
                }
                DebouncedEvent::Remove(p) => {
                    println!("Remove {:?}", p)
                }
                DebouncedEvent::NoticeRemove(p) => {
                    println!("NoticeRemove {:?}", p)
                }
                DebouncedEvent::Rename(a, b) => {
                    println!("Rename {:?} -> {:?}", a, b)
                }
                _ => (),
            },
            Err(e) => println!("watch error: {:?}", e),
        };
    }
}

fn save_watcher(
    sync_dir: &str,
    file_scan_tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
    file_add_rx: std::sync::mpsc::Receiver<HashmapCmd>,
) {
    let mut watcher = watcher(file_scan_tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<String, Savedata> = HashMap::new();
    loop {
        match file_add_rx.recv().unwrap() {
            HashmapCmd::WatchFile(savefile) => {
                watcher.watch(&savefile.file, RecursiveMode::NonRecursive);
            }
            HashmapCmd::WatchDir(saveloc) => {
                // this is what makes events bubble up for file modification
                watcher.watch(&saveloc.dir, RecursiveMode::Recursive);
            }
            HashmapCmd::Unwatch(rmpath) => {
                // watcher.unwatch(&entry).unwrap();
            }
            HashmapCmd::Copy(src) => {
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

// find all files in @json_dir that end in .json, return a vector of SaveDir's from them
fn get_save_descriptors(json_dir: &str) -> (Vec<SaveDir>, Vec<SaveFile>) {
    let json_dir = helper::strip_quotes(json_dir);
    let mut dir_accu: Vec<SaveDir> = vec![];
    let mut file_accu: Vec<SaveFile> = vec![];

    for entry in WalkDir::new(json_dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let f_name = entry.file_name().to_string_lossy();

        if f_name.ends_with(".json") {
            let entry = entry.path().to_str().unwrap();
            parse_save_json(&entry, &mut dir_accu, &mut file_accu);
            // file_add_tx.send(entry.clone().to_string());
        }
    }
    (dir_accu, file_accu)
}

fn get_save_watch_entries(savelocs: &Vec<SaveDir>) -> Vec<String> {
    let mut save_watch_entries: Vec<String> = vec![];
    // find all save files in each directory
    // add things to the hash map based on the settings from the savelocs
    for e in savelocs.iter() {
        for entry in WalkDir::new(&e.dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
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

// crawl through saves listed from save files and send results to watcher thread
fn find_saves(json_dir: &str, file_add_tx: &mpsc::Sender<HashmapCmd>) {
    let (savelocs, savefiles) = get_save_descriptors(json_dir);
    // let save_watch_entries = get_save_watch_entries(&savelocs);
    for e in savelocs {
        // println!("{}", e.dir);
        file_add_tx.send(HashmapCmd::WatchDir(e));
    }

    for e in savefiles {
        // println!("{}", e.dir);
        file_add_tx.send(HashmapCmd::WatchFile(e));
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
