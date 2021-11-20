/*
last thing I did was write something that will find all files in a directory and now I want to make a big hash table
(this is a bad idea and isn't memory efficient) that keys the sha of every save back to an object that stores the
filepath to every version of the save fileso that their update times can be checked and the most recent one can be
propageted to all sources

note: are there any common file systems that don't use last modified?
*/

use walkdir::WalkDir;
use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::{Duration};
use sha2::{Digest, Sha256};
use std::fs;
use std::collections::{HashMap, HashSet};

struct Savefile {
    result: String,
}

fn do_links() {
    let mario_tuple = vec![
        ("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).gba",
            "/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).gba"),
        ("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).sav",
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
    let metadata = std::fs::metadata("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).gba").unwrap();

    println!("{:?}", metadata.file_type());
}
fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

fn file_sha256(path: &str) -> Savefile {
    let bytes = std::fs::read(path).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let s = Savefile {
        // TODO: this produces a string like "[A3, FE, ..., E3]", but hey it's accurate. if this thing works i'll fix it
        result: format!("{:02X?}", hasher.finalize()),
    };
    s
}

fn find_savs() {
    let walkdir = "/home/alex/Dropbox/sync";
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();
    let mut save_map/* : HashMap<&str, HashSet<&str>> */ = HashMap::new();

    for entry in WalkDir::new(walkdir) .follow_links(true) .into_iter() .filter_map(|e| e.ok()) {
        let f_name = entry.file_name().to_string_lossy();
        let sec = entry.metadata().unwrap().modified().unwrap();

        // right now having two files registered copied on top of each other will ping pong back and forth which is why
        // it's important to set up hashing to ensure that files that are currently being tracked aren't inserted into
        // the system as current
        if f_name.ends_with(".ss1") /* || f_name.ends_with(".ss2") || f_name.ends_with(".ss3") */ {
            println!("{}", entry.path().to_str().unwrap());
            watcher.watch(entry.path().to_str().unwrap(), RecursiveMode::Recursive).unwrap();
        }
    }

    loop {
        match rx.recv() {
            Ok(event) =>
                match event {
                    DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                        println!("Update: {:?}", p);
                        let incage =  std::fs::metadata(&p).unwrap().modified().unwrap();
                        let compage = std::fs::metadata("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).ss2")
                            .unwrap().modified().unwrap();
                        let p_str = p.clone().into_os_string().into_string().unwrap();
                        if let Ok(mut file) = fs::File::open(&p_str) {
                            let res = file_sha256(&p_str);
                            // println!("{:?}", res.result);
                            // https://stackoverflow.com/a/65550108
                            match save_map.get(&res.result) {
                                Some(v) => ()/* {
                                    println!("yes existy {:?}", v);
                                } */,
                                None => {
                                    let myset: HashSet<&str> = HashSet::new();
                                    save_map.insert(&res.result, myset);
                                    println!("insert {:?}", p_str);
                                },
                            }
                        }
                        // if incage > compage {
                        //     std::fs::copy(&p, "/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).ss3");
                        //     println!("update {:?}", p);
                        // }
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

fn main() {
    // do_links();
    // get_metadata();
    find_savs();
}
