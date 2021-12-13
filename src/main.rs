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
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use structopt::StructOpt;
use walkdir::WalkDir;

static COPY_PATH: &str = "/home/alex/Dropbox/sync";
static EMU_PATH: &str =  "/home/alex/Dropbox/rand";

#[derive(StructOpt)]
struct Cli {
    #[structopt(default_value = "settings.json")]
    settings: std::path::PathBuf,
}

struct Savedata {
    filemap: HashMap<String, String>,
}

struct Globals {
    rx: mpsc::Receiver<notify::DebouncedEvent>,
    watcher: notify::inotify::INotifyWatcher,
    save_map: HashMap<String, Savedata>,
}

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

fn find_savs(file_add_tx: &mpsc::Sender<String>) {
    let walkdir = "/home/alex/Dropbox/rand";

    for entry in WalkDir::new(walkdir) .follow_links(true) .into_iter() .filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();
        let sec = entry.metadata().unwrap().modified().unwrap();

        if f_name.ends_with(".txt") {
            let entry = entry.path().to_str().unwrap();
            println!("file_add_tx -> {:?}", entry);
            file_add_tx.send(entry.clone().to_string());
        }
    }
}

fn setup() {
    fs::create_dir_all(COPY_PATH).unwrap();
    fs::create_dir_all(EMU_PATH).unwrap();
}

/*
does initial scan
performs file copy back and forth
decides upon watch/unwatch events (only)
sends file events to thread 1
*/
// say this is done? just looks for file updates and informs save watcher
fn save_scanner(file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
                file_add_tx:  mpsc::Sender<String>) -> Result<()> {
    find_savs(&file_add_tx);
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    // println!("{:?}", p);
                    let entry = p.to_str().unwrap();
                    file_add_tx.send(entry.clone().to_string());
                }
                DebouncedEvent::NoticeWrite(p) => println!("NoticeWrite {:?}", p),
                DebouncedEvent::Create(p) => println!("Create {:?}", p),
                DebouncedEvent::Remove(p) => println!("Remove {:?}", p),
                DebouncedEvent::NoticeRemove(p)  => println!("NoticeRemove {:?}", p),
                DebouncedEvent::Rename(a, b) => println!("Rename {:?} -> {:?}", a, b),
                _ => (),
           },
           Err(e) => println!("watch error: {:?}", e),
        };
    }
}

/*
responds to file update events
no file i/o
owns save_map
writes to watcher
decides when savegame parity should be updated
*/
fn save_watcher(file_scan_tx: std::sync::mpsc::Sender<notify::DebouncedEvent>,
                file_add_rx:  std::sync::mpsc::Receiver<String>,) {
    let mut watcher = watcher(file_scan_tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<String, Savedata> = HashMap::new();
    loop {
        let add_path = file_add_rx.recv().unwrap();
        watcher.watch(&add_path, RecursiveMode::Recursive).unwrap();
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
        // watcher.unwatch(&entry).unwrap();
        // if file is in save_map, do watch add, else do watch remove
    }
}

fn parse_args(p: &std::path::PathBuf) -> Result<Value> {
    let bytes = std::fs::read_to_string(p).unwrap();
    serde_json::from_str(&bytes)
}

fn main() {
    /*
    start listener, wait for it to finish init (prob actually dont need to wait since channel has finished init)
    crawl directories, send messages to listener
    */
    let args = Cli::from_args();
    let parse = parse_args(&args.settings).unwrap();
    let tracker_dir = parse["tracker_dir"].to_string();

    let (file_scan_tx, file_scan_rx) = mpsc::channel();
    let (file_add_tx,  file_add_rx) =  mpsc::channel();

    setup();

    let save_scanner_handle = thread::spawn(move || {
        save_scanner(file_scan_rx, file_add_tx);
    });
    let save_watcher_handle = thread::spawn(move || {
        save_watcher(file_scan_tx, file_add_rx);
    });

    save_scanner_handle.join().unwrap();
    save_watcher_handle.join().unwrap();
}
