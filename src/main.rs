use notify::{DebouncedEvent, Watcher, RecursiveMode, watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use std::os::unix::fs;
// use std::fs;

fn watch_files() {
    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    watcher.watch("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).sav", RecursiveMode::Recursive).unwrap();
    watcher.watch("/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).sav", RecursiveMode::Recursive).unwrap();

    loop {
        match rx.recv() {
            Ok(event) =>
                match event {
                    DebouncedEvent::NoticeWrite(p) | DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                        println!("Update: {:?}", p);
                    }
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
        ("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).gba", "/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).gba"),
        ("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).sav", "/home/alex/Mario & Luigi - Superstar Saga (USA, Australia).sav"),
    ];
    for link in mario_tuple.iter() {
        fs::symlink(link.0, link.1);
    }
}

// take a directory. if "sync", ensure all files are up to date between the two folders. if "symlink", copy the original
// file to the sync folder and replace the original with a symlink (optimization, start with copy mode only?).

fn get_metadata() {
    let metadata = std::fs::metadata("/home/alex/Dropbox/sync/Mario & Luigi - Superstar Saga (USA, Australia).gba").unwrap();

    println!("{:?}", metadata.file_type());
}

fn main() {
    do_links();
    watch_files();
    // get_metadata();
}
