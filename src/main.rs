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
use std::collections::HashMap;
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
                find_saves(json_dir, file_add_tx);
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
}

enum SaveOpts {
    File(SaveFile),
    Dir(SaveDir),
}

struct SaveDef {
    path: PathBuf,
    options: SaveOpts,
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
                dir: PathBuf::from("/"),
                filetypes: vec![],
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
                // let mut dst = PathBuf::from(sync_dir);
                // let src_file_name = src.file_name().expect("this was probably not a file");
                // dst.push(&src_file_name);
                // dst.set_extension(src.extension().unwrap());
                // println!("copy {:?} into save manager at {:?}", src, dst);
                // std::fs::create_dir_all(&dst);
                // std::fs::copy(&src, &dst);
            }
        }
    }
}

// find all files in @json_dir that end in .json, return a vector of SaveDir's from them
fn get_save_descriptors(json_dir: &str) -> Vec<SaveDef> {
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

// crawl through saves listed from save files and send results to watcher thread
fn find_saves(json_dir: &str, file_add_tx: &mpsc::Sender<HashmapCmd>) {
    let saves = get_save_descriptors(json_dir);

    for e in saves {
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
