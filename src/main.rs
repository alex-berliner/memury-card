use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::{Duration, SystemTime};

fn watch_files() {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(1)).unwrap();

    watcher.watch("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).sav", RecursiveMode::Recursive).unwrap();
    watcher.watch("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).ss1", RecursiveMode::Recursive).unwrap();
    // watcher.watch("/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).sav", RecursiveMode::Recursive).unwrap();
    // watcher.watch("/home/alex/Code/savesync/testfiles/old.txt", RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
            Ok(event) =>
                match event {
                    DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                        println!("Update: {:?}", p);
                        let incage =  std::fs::metadata(&p).unwrap().modified().unwrap();
                        let compage = std::fs::metadata("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).ss2")
                            .unwrap().modified().unwrap();
                        if incage > compage {
                            std::fs::copy(&p, "/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).ss3");
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

fn main() {
    // do_links();
    watch_files();
    // get_metadata();
}
