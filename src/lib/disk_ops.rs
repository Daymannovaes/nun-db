use std::collections::HashMap;
use std::fs::{create_dir_all, read_dir, File};
use std::path::Path;
use std::sync::Arc;

use std::sync::mpsc::{channel, Receiver, Sender};

use bo::*;
use db_ops::*;

const SNAPSHOT_TIME: i64 = 30000;
const FILE_NAME: &'static str = "freira-db.data";
const DIR_NAME: &'static str = "dbs/";

pub fn load_db_from_disck_or_empty(name: String) -> HashMap<String, String> {
    println!("Will read the database {} from disk", name);
    let mut initial_db = HashMap::new();
    let db_file_name = file_name_from_db_name(name.clone());
    if Path::new(&db_file_name).exists() {
        // May I should move this out of here
        let mut file = File::open(db_file_name).unwrap();
        initial_db = bincode::deserialize_from(&mut file).unwrap();
    }
    return initial_db;
}

fn load_one_db_from_disk(dbs: &Arc<Databases>, entry: std::io::Result<std::fs::DirEntry>) {
    let mut dbs = dbs.map.lock().unwrap();
    if let Ok(entry) = entry {
        let full_name = entry.file_name().into_string().unwrap();
        let splited_name: Vec<&str> = full_name.split("-").collect();
        let db_name = splited_name.first().unwrap();
        let db_data = load_db_from_disck_or_empty(db_name.to_string());
        dbs.insert(
            db_name.to_string(),
            create_db_from_hash(db_name.to_string(), db_data),
        );
    }
}
fn load_all_dbs_from_disk(dbs: &Arc<Databases>) {
    if let Ok(entries) = read_dir(DIR_NAME) {
        for entry in entries {
            load_one_db_from_disk(dbs, entry);
        }
    }
}
// send the given database to the disc
pub fn file_name_from_db_name(db_name: String) -> String {
    format!(
        "{dir}/{db_name}-{sufix}",
        dir = DIR_NAME,
        db_name = db_name,
        sufix = FILE_NAME
    )
}

fn storage_data_disk(db: &Database, db_name: String) {
    let db = db.map.lock().unwrap();
    let mut file = File::create(file_name_from_db_name(db_name)).unwrap();
    bincode::serialize_into(&mut file, &db.clone()).unwrap();
}

// calls storage_data_disk each $SNAPSHOT_TIME seconds
pub fn start_snap_shot_timer(timer: timer::Timer, dbs: Arc<Databases>) {
    println!("Will start_snap_shot_timer");
    load_all_dbs_from_disk(&dbs);
    match create_dir_all(DIR_NAME) {
        Ok(_) => {}
        Err(e) => {
            println!("Error creating the data dirs {}", e);
            panic!("Error creating the data dirs");
        }
    };
    let (_tx, rx): (Sender<String>, Receiver<String>) = channel();
    let _guard = {
        timer.schedule_repeating(chrono::Duration::milliseconds(SNAPSHOT_TIME), move || {
            let dbs = dbs.map.lock().unwrap();
            for (database_name, db) in dbs.iter() {
                println!("Will snapshot the database {}", database_name);
                storage_data_disk(db, database_name.clone());
            }
        })
    };
    rx.recv().unwrap(); // Thread will run for ever
}