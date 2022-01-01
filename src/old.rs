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

fn channel_experiment() {
    struct txxxx {
        x: i32,
        a: i32,
        b: i32,
        c: i32,
    }

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let x  = txxxx {
            x: 0,
            a: 0,
            b: 0,
            c: 0,
        };
        // let val = String::from("hi");
        tx.send(x).unwrap();
    });

    let received = rx.recv().unwrap();
    println!("Got: {}", received.a);
}


fn listen(globals: &mut Globals, rx: mpsc::Receiver<String>) {
    let mut save_map: HashMap<String, Savedata> = HashMap::new();

    for received in rx {
        println!("RXRXRXRRX {:?}", received);
    }

    loop {
        match globals.rx.recv() {
            Ok(event) =>
            match event {
                DebouncedEvent::Write(p) | DebouncedEvent::Chmod(p) => {
                    let p = &p.into_os_string().into_string().unwrap();
                    let new_hash = file_sha256(p);
                    let path_split: Vec<&str> = p.split("/").collect();
                    let fname = path_split.last().unwrap();
                    let hs = globals.save_map.get(*fname).unwrap();
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


// fn find_saves(file_add_tx: &mpsc::Sender<String>) {
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


fn inner_map() {
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
    for (key, value) in &hs.filemap {
        let real_hash = file_sha256(key);
        if add_hash != real_hash {
            println!("update {:?} -> {:?}", add_path, key);
            std::fs::copy(&add_path, key);
        } else {
            println!("no copy");
        }
    }
    // if file is in save_map, do watch add, else do watch remove
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
