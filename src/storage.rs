use chrono::{DateTime, Utc};
use std::time::SystemTime;
use std::fs;
use std::path::Path;

static STORAGE_DIRECTORY: &str = "./data_storage";

pub fn generate_file_name(group: &str, entity: &str) -> String {
    let system_time =SystemTime::now();
    let now: DateTime<Utc> = system_time.into();
    ensure_group_and_entity_dir(group, entity);
    return format!("./{}/{}/{}/{}.lua", STORAGE_DIRECTORY, group, entity, now.to_rfc3339())
}

pub fn ensure_dir() {
    let file_path = &format!("./{}", STORAGE_DIRECTORY);
    let dir_path = Path::new(file_path);
    if !dir_path.exists() {
        fs::create_dir(dir_path).unwrap()
    }
}

fn ensure_group_dir(group: &str) {
    let file_path = &format!("./{}/{}", STORAGE_DIRECTORY, group);
    let dir_path = Path::new(file_path);
    if !dir_path.exists() {
        fs::create_dir(dir_path).unwrap()
    }
}

fn ensure_group_and_entity_dir(group: &str, entity: &str) {
    ensure_group_dir(group);
    let file_path = &format!("./{}/{}/{}", STORAGE_DIRECTORY, group, entity);
    let dir_path = Path::new(file_path);
    if !dir_path.exists() {
        fs::create_dir(dir_path).unwrap()
    }
}

pub fn collect_files(group: &str, entity: &str) -> Vec<String> {
    ensure_group_and_entity_dir(group, entity);
    let file_path = &format!("./{}/{}/{}", STORAGE_DIRECTORY, group, entity);
    let dir_path = Path::new(file_path);
    let read_dir = std::fs::read_dir(dir_path).unwrap();
    read_dir.filter_map(|x| -> Option<String> {
        if x.is_err() { return None }
        let file_name = x.unwrap().file_name().into_string();
        if file_name.is_err() { return None }
        let unwrapped_value = file_name.unwrap();
        if unwrapped_value.ends_with(".lua") {return Some(unwrapped_value)}
        return None
    }).collect()
}