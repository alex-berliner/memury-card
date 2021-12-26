/*
versioned storage area: this app currently just keeps files in sync as they're updated. the true purpose is to maintain
a primary area where saves will be located. saves that don't exist in this area will be copied into it. the program will
on startup/periodically/continuously/when responding to events crawl monitored directories to find saves that the user
has in their save directories that should be copied into the storage area
two saves with the same name will be kept in sync with the program copying the more recent save on top of the old save.

resolution mechanism: how should the file of record be determined? people will not like it if i blast away their save
files of the same name in different folders on the first run. there may even be games where the same name is used in
different folders to refer to different saves. also during startup it should probably take the date of all the files and
use the last modified and copy that to all the other attached files. "attachment" is going to have to be determined
somehow and it's not simple. i could let the user define where different files should be attached or the attachment
scheme for certain directories. a principle for this program is centralized management so they could be able to declare
the type of attachment as they define what directory to look for saves in

high configuribility: up until now I've been proofing things out but a lot of how I want this program to work is outside
of code. I want to build this so that it's easily manageable from configuration files, not inside a horrible form-driven
program. Directories to manage and their management style will be pulled from json file(s) that define where the program
should look, for what it should look, how it should resolve save conflicts, etc.

note: are there any common file systems that don't use last modified?
*/

use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use serde_json::{Result, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::WalkDir;
use std::path::{PathBuf, Path};
#[derive(StructOpt)]
struct Cli {
    #[structopt(default_value = "settings.json")]
    settings: std::path::PathBuf,
}

struct Savedata {
    filemap: HashMap<String, String>,
}

#[allow(dead_code)]
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn file_sha256(path: &str) -> String {
    // TODO: check file exists
    // TODO: make better hash format
    let bytes = std::fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:02X?}", hasher.finalize())
}

// fn find_savs(file_add_tx: &mpsc::Sender<String>) {
//     let walkdir = "/home/alex/Dropbox/rand";

//     for entry in WalkDir::new(walkdir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
//         let f_name = entry.file_name().to_string_lossy();

//         if f_name.ends_with(".txt") {
//             let entry = entry.path().to_str().unwrap();
//             println!("file_add_tx -> {:?}", entry);
//             file_add_tx.send(entry.clone().to_string());
//         }
//     }
// }

/*
does initial scan
performs file copy back and forth
decides upon watch/unwatch events (only)
sends file events to thread 1
*/
// say this is done? just looks for file updates and informs save watcher
fn save_scanner(json_dir: &str,
                file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
                file_add_tx: &mpsc::Sender<HashmapCmd>) -> Result<()> {
    // find_saves(json_dir, file_add_tx);
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    println!("{:?}", p);
                    let entry = p.to_str().unwrap();
                    file_add_tx.send(HashmapCmd::Copy(entry.clone().to_string()));
                }
                DebouncedEvent::NoticeWrite(p) => { println!("NoticeWrite {:?}", p) }
                DebouncedEvent::Create(p) => { println!("Create {:?}", p) }
                DebouncedEvent::Remove(p) => { println!("Remove {:?}", p) }
                DebouncedEvent::NoticeRemove(p)  => { println!("NoticeRemove {:?}", p) }
                DebouncedEvent::Rename(a, b) => { println!("Rename {:?} -> {:?}", a, b) }
                _ => { () }
           },
           Err(e) => println!("watch error: {:?}", e),
        };
    }
}

enum HashmapCmd {
    Watch(String),
    Unwatch(String),
    Copy(String),
}

/*
responds to file update events
no file i/o
owns save_map
writes to watcher
decides when savegame parity should be updated
*/
fn save_watcher(sync_dir: &str,
                file_scan_tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
                file_add_rx:  std::sync::mpsc::Receiver<HashmapCmd>,) {
    let mut watcher = watcher(file_scan_tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<String, Savedata> = HashMap::new();
    loop {
        match file_add_rx.recv().unwrap() {
            HashmapCmd::Watch(add_path) => {
                // this is what makes events bubble up for file modification
                watcher.watch(&add_path, RecursiveMode::Recursive);
                let add_hash = file_sha256(&add_path);
                // println!("{:?} {:?}", add_path, add_hash);
                let path_split: Vec<&str> = add_path.split("/").collect();
                let fname = path_split.last().unwrap();
                let inner_map = save_map.entry(fname.to_string()).or_insert_with(||{Savedata{filemap: HashMap::new()}});
                inner_map.filemap.entry(add_path.clone()).or_insert(add_hash.to_string());
                // println!("out: {:?} in: {:?}", add_path, inner_map.filemap.get(&add_path).unwrap());
                let hs = save_map.get(*fname).unwrap();
                // TODO: a file update is n^2 because it triggers "no copy" checks on each other file. can be
                // fixed by caching the hash of the last saved value and not doing anything if the hash is the
                // same
                // for (key, value) in &hs.filemap {
                //     let real_hash = file_sha256(key);
                //     if add_hash != real_hash {
                //         println!("update {:?} -> {:?}", add_path, key);
                //         std::fs::copy(&add_path, key);
                //     } else {
                //         println!("no copy");
                //     }
                // }
                // if file is in save_map, do watch add, else do watch remove
            }
            HashmapCmd::Unwatch(rmpath) => {
                // watcher.unwatch(&entry).unwrap();
            }
            HashmapCmd::Copy(src) => {
                let mut dst = PathBuf::new();
                let src = Path::new(&src);
                dst.push(sync_dir);
                dst.push(src.file_name().expect("this was probably not a file"));
                dst.set_extension("txt");
                println!("copy {:?} into save manager at {:?}", src, dst);
                std::fs::copy(&src, dst);
            }
        }
    }
}

struct SaveLoc {
    dir: String,
    resolution_strategy: String,
    filetypes: Vec<String>,
}

impl SaveLoc {
    #[allow(dead_code)]
    fn print(&self) {
        println!("dir:      {}", self.dir);
        println!("reso:     {}", self.resolution_strategy);
        for j in 0 .. self.filetypes.len() {
            println!("filetype: {}", self.filetypes[j]);
        }
    }
}

fn parse_json(p: &std::path::PathBuf) -> Result<Value> {
    let bytes = std::fs::read_to_string(p).unwrap();
    serde_json::from_str(&bytes)
}

fn strip_quotes(s: &str) -> String{
    let s = s.to_string();
    // TODO: this doesn't do what the function says it does
    s.trim_matches('"').to_string()
}

fn parse_save_json(json_file: &str, accu: &mut Vec<SaveLoc>) {
    let bytes = std::fs::read_to_string(json_file).unwrap();
    let json: Value = serde_json::from_str(&bytes).unwrap();
    let save_areas = json["save_areas"].as_array().unwrap();

    for i in 0 .. save_areas.len() {
        let mut save = SaveLoc {
            dir: strip_quotes(save_areas[i]["dir"].as_str().unwrap()),
            resolution_strategy: strip_quotes(save_areas[i]["resolution_strategy"].as_str().unwrap()),
            filetypes: vec![],
        };
        let filetypes = save_areas[i]["filetypes"].as_array().unwrap();
        for j in 0 .. filetypes.len() {
            save.filetypes.push(filetypes[j].as_str().unwrap().to_string());
        }
        accu.push(save);
    }
}

// find all files in @json_dir that end in .json, return a vector of SaveLoc's from them
fn get_save_locs(json_dir: &str) -> Vec<SaveLoc> {
    let json_dir = strip_quotes(json_dir);
    let mut accu: Vec<SaveLoc> = vec![];

    for entry in WalkDir::new(json_dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();

        if f_name.ends_with(".json") {
            let entry = entry.path().to_str().unwrap();
            parse_save_json(&entry, &mut accu);
            // file_add_tx.send(entry.clone().to_string());
        }
    }
    accu
}

fn get_save_watch_entries(savelocs: &Vec<SaveLoc>) -> Vec<String> {
    let mut save_watch_entries: Vec<String> = vec![];
    // find all save files in each directory
    // add things to the hash map based on the settings from the savelocs
    for e in savelocs.iter() {
        for entry in WalkDir::new(&e.dir).follow_links(true).into_iter().filter_map(|e| e.ok()) {
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
fn find_saves(  json_dir: &str,
                file_add_tx: &mpsc::Sender<HashmapCmd>) {
    let savelocs = get_save_locs(json_dir);
    let save_watch_entries = get_save_watch_entries(&savelocs);
    for e in save_watch_entries {
        println!("{}", e);
        file_add_tx.send(HashmapCmd::Watch(e));
    }
}

fn interactive( json_dir: &str,
                file_add_tx: &mpsc::Sender<HashmapCmd>) {
    loop {
        println!("Enter command: ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        match input {
            "s" => {
                find_saves(json_dir, file_add_tx);
            },
            _ => ()
        }
    }
}

fn main() {
    let args = Cli::from_args();
    let parse = parse_json(&args.settings).unwrap();
    let tracker_dir1 = parse["tracker_dir"].to_string();
    let tracker_dir2 = tracker_dir1.clone();

    let sync_dir = strip_quotes(&parse["sync_dir"].to_string());

    let (file_scan_tx, file_scan_rx) = mpsc::channel();
    let (file_add_tx,  file_add_rx) =  mpsc::channel();
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
