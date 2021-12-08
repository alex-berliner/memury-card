/*
versioned storage area: this app currently just keeps files in sync as they're updated. the true purpose is to maintain
a primary area where saves will be located. saves that don't exist in this area will be copied into it. the program will
on startup/periodically/continuously/when responding to events crawl monitored directories to find saves that the user
has in their save directories that should be copied into the storage area
two saves with the same name will be kept in sync with the program copying the more recent save on top of the old save.

important detour: threading. currently the listen() function isn't threaded which will probably become a problem if I
want the program to do more things or just handle different operations at the same time like updating entries while
continuing to listen for savegame changes. so listen(), which for savegames looks for updates and maintains parity must
be threaded, along with, in the future, the things that will be manipulating which savegames are being watched, ui, idk.
globals.watcher stores a path to every file currently being kept track of, as does globals.save_map. A channel is
probably the best way to funnel updates to the watcher, but do more things need access to save_map besides just the one
thread? probably not, if this were a module it would be just used for maintaining parity and it could probably be stored
in the thread.

note: are there any common file systems that don't use last modified?
*/

use walkdir::WalkDir;
use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use std::sync::mpsc;
use std::time::{Duration};
use sha2::{Digest, Sha256};
use std::fs;
use std::collections::{HashMap, HashSet};
use std::thread;

static COPY_PATH: &str = "/home/alex/Dropbox/sync";
static EMU_PATH: &str =  "/home/alex/Dropbox/rand";

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

        // right now having two files registered copied on top of each other will ping pong back and forth which is why
        // it's important to set up hashing to ensure that files that are currently being tracked aren't inserted into
        // the system as current
        if f_name.ends_with(".txt") {
            // TODO: this needs to hash on filename and catalog the different versions of the file instead of what it
            // does right now
            let entry = entry.path().to_str().unwrap();
            println!("file_add_tx -> {:?}", entry);
            // let inner_map = save_map.entry(f_name.to_string()).or_insert_with(||{Savedata{filemap: HashMap::new()}});
            // inner_map.filemap.insert(entry.to_string(), res.to_string());
            file_add_tx.send(entry.clone().to_string());
        }
    }
}

fn setup_watch(globals: &mut Globals) {
    // https://stackoverflow.com/a/45724688
    for (ki, vi) in globals.save_map.iter() {
        let hs = globals.save_map.get(ki).unwrap();
        for (kj, vj) in &hs.filemap {
            // https://docs.rs/notify/4.0.17/notify/trait.Watcher.html#tymethod.unwatch
            &globals.watcher.watch(kj, RecursiveMode::Recursive).unwrap();
            println!("Watching {:?}", kj);
        }
    }
}

fn setup() {
    fs::create_dir_all(COPY_PATH).unwrap();
    fs::create_dir_all(EMU_PATH).unwrap();
}

fn create_globals() -> Globals {
    let (tx, rx) = mpsc::channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    let mut save_map: HashMap<String, Savedata> = HashMap::new();
    let globals = Globals {
        rx: rx,
        watcher: watcher,
        save_map: save_map,
    };
    globals
}

/*
does initial scan
performs file copy back and forth
decides upon watch/unwatch events (only)
sends file events to thread 1
*/
// say this is done? just looks for file updates and informs save watcher
fn save_scanner(file_scan_rx: mpsc::Receiver<notify::DebouncedEvent>,
                file_add_tx:  mpsc::Sender<String>) {
    find_savs(&file_add_tx);
    loop {
        match file_scan_rx.recv() {
            Ok(event) => match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    println!("{:?}", p);
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
        let entry = file_add_rx.recv().unwrap();
        let new_hash = file_sha256(&entry);
        println!("{:?} {:?}", entry, new_hash);
        let path_split: Vec<&str> = entry.split("/").collect();
        let fname = path_split.last().unwrap();
        // let hs = save_map.get(*fname).unwrap();
        // TODO: a file update is n^2 because it triggers "no copy" checks on each other file. can be
        // fixed by caching the hash of the last saved value and not doing anything if the hash is the
        // same
        // for (key, value) in &hs.filemap {
        //     // println!("------------> {} / {}", key, value);
        //     let real_hash = file_sha256(key);
        //     // println!("real hash: {:?}", real_hash);
        //     if new_hash != real_hash {
        //         println!("must copy\n{:?}\n{:?}", new_hash, real_hash);
        //         println!("{:?}", key);
        //         std::fs::copy(&entry, key);
        //     } else {
        //         println!("no copy");
        //     }
        // }
// watcher.watch(&entry, RecursiveMode::Recursive).unwrap();
        // watcher.unwatch(&entry).unwrap();
        // if file is in save_map, do watch add, else do watch remove
    }
}

fn main() {
    /*
    start listener, wait for it to finish init (prob actually dont need to wait since channel has finished init)
    crawl directories, send messages to listener
    */
    let (file_scan_tx, file_scan_rx) = mpsc::channel();
    let (file_add_tx,  file_add_rx) =  mpsc::channel();
    setup();

    print_type_of(&file_scan_tx);
    let save_scanner_handle = thread::spawn(move || {
        save_scanner(file_scan_rx, file_add_tx);
    });
    let save_watcher_handle = thread::spawn(move || {
        save_watcher(file_scan_tx, file_add_rx);
    });

    save_scanner_handle.join().unwrap();
    save_watcher_handle.join().unwrap();
}
