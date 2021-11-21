/*
versioned storage area: this app currently just keeps files in sync as they're updated. the true purpose is to maintain
a primary area where saves will be located. saves that don't exist in this area will be copied into it. the program will
on startup/periodically/continuously/when responding to events crawl monitored directories to find saves that the user
has in their save directories that should be copied into the storage area
two saves with the same name will be kept in sync with the program copying the more recent save on top of the old save.

note: are there any common file systems that don't use last modified?
*/

use walkdir::WalkDir;
use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::{Duration};
use sha2::{Digest, Sha256};
use std::fs;
use std::collections::{HashMap, HashSet};

struct Savedata {
    filemap: HashMap<String, String>,
}

fn do_links() {
    let mario_tuple = vec![
        ("/home/alex/Dropbox/rand/Mario & Luigi - Superstar Saga (USA, Australia).gba",
            "/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).gba"),
        ("/home/alex/Dropbox/rand/Mario & Luigi - Superstar Saga (USA, Australia).sav",
            "/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).sav"),
    ];
    for link in mario_tuple.iter() {
        std::os::unix::fs::symlink(link.0, link.1);
    }
}

// take a directory. if "sync", ensure all files are up to date between the two folders. if "symlink", copy the original
// file to the sync folder and replace the original with a symlink (optimization, start with copy mode only?).
fn get_metadata() {
    // file scanner: look for files of specified file types in targeted directories. if found, add by copy to versioned
    // storage area. when adding to versioned storage area, use name to detect duplicates. if possible, use save header
    // to detect game instead for better management (defer).
    // file watcher: establish link between all files with the same name(?) and watch for changes on each copy. when one
    // copy is updated, propagate the change to all other copies. prompt for overwriting one source with another to
    // avoid data deletion. only perform overwrite if hashes of two files are different
    // A <----> B
    // old      new
    // compare with metadata.age_or_something()
    // propagate data from new to old by copying into versioned storage area
    // check if old data is in use (defer)
    // sort by having save file header checks (defer)
    // json file managed directory structure (defer)
    let metadata = std::fs::metadata("/home/alex/Dropbox/rand/Mario & Luigi - Superstar Saga (USA, Australia).gba").unwrap();

    println!("{:?}", metadata.file_type());
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

fn find_savs(save_map:&mut HashMap<String, Savedata>) {
    let walkdir = "/home/alex/Dropbox/rand";

    for entry in WalkDir::new(walkdir) .follow_links(true) .into_iter() .filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();
        let sec = entry.metadata().unwrap().modified().unwrap();

        // right now having two files registered copied on top of each other will ping pong back and forth which is why
        // it's important to set up hashing to ensure that files that are currently being tracked aren't inserted into
        // the system as current
        if f_name.ends_with(".ss1") /* || f_name.ends_with(".ss2") || f_name.ends_with(".ss3") */ {
            // TODO: this needs to hash on filename and catalog the different versions of the file instead of what it
            // does right now
            let entry = entry.path().to_str().unwrap();
            let res = file_sha256(&entry);
            // println!("entry  {:?}", entry);
            // println!("res    {:?}", res);
            // println!("f_name {:?}", f_name);
            let inner_map = save_map.entry(f_name.to_string()).or_insert_with(||{Savedata{filemap: HashMap::new()}});
            inner_map.filemap.insert(entry.to_string(), res.to_string());
        }
    }
}

fn listen(save_map:&mut HashMap<String, Savedata>) {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    // https://stackoverflow.com/a/45724688
    for (ki, vi) in &*save_map {
        let hs = save_map.get(ki).unwrap();
        for (kj, vj) in &hs.filemap {
            watcher.watch(kj, RecursiveMode::Recursive).unwrap();
            println!("Watching {:?}", kj);
        }
    }

    loop {
        match rx.recv() {
            Ok(event) =>
                match event {
                    DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                        let p = &p.into_os_string().into_string().unwrap();
                        // println!("Update: {:?}", p);
                        let new_hash = file_sha256(p);
                        // println!("new_hash: {:?}", new_hash);
                        let path_split: Vec<&str> = p.split("/").collect();
                        let fname = path_split.last().unwrap();
                        let hs = save_map.get(*fname).unwrap();
                        // TODO: a file update is n^2 because it triggers "no copy" checks on each other file. can be
                        // fixed by caching the hash of the last saved value and not doing anything if the hash is the
                        // same
                        for (key, value) in &hs.filemap {
                            // println!("------------> {} / {}", key, value);
                            let real_hash = file_sha256(key);
                            // println!("real hash: {:?}", real_hash);
                            if new_hash != real_hash {
                                println!("must copy\n{:?}\n{:?}", new_hash, real_hash);
                                println!("{:?}", key);
                                std::fs::copy(&p, key);
                            } else {
                                println!("no copy");
                            }
                        }
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

fn setup() {
    let home_dir = "/home/alex/Dropbox/sync";
    fs::create_dir_all(&home_dir).unwrap();
}

fn main() {
    let mut save_map = HashMap::new();
    // do_links();
    // get_metadata();
    setup();
    find_savs(&mut save_map);
    listen(&mut save_map);
}
