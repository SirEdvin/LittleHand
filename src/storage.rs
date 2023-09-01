use chrono::{DateTime, Utc};
use rocket::fs::NamedFile;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;
use tokio::fs as tokio_fs;

static STORAGE_DIRECTORY: &str = "data_storage";
pub static DEFAULT_FILE: &str = "1990-01-01-01-01-01.lua";

pub fn generate_file_name(group: &str, entity: &str) -> String {
    let system_time = SystemTime::now();
    let now: DateTime<Utc> = system_time.into();
    ensure_group_and_entity_dir(group, entity);
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    buf.push(group);
    buf.push(entity);
    buf.push(now.format("%Y-%m-%d-%H-%M-%S.lua").to_string());
    return buf.to_string_lossy().to_string();
}

pub fn ensure_dir() {
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    if !buf.exists() {
        fs::create_dir(buf).unwrap()
    }
}

fn ensure_group_dir(group: &str) {
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    buf.push(group);
    if !buf.exists() {
        fs::create_dir(buf).unwrap()
    }
}

fn ensure_group_and_entity_dir(group: &str, entity: &str) {
    ensure_group_dir(group);
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    buf.push(group);
    buf.push(entity);
    if !buf.exists() {
        fs::create_dir(buf).unwrap()
    }
}

pub fn collect_files(group: &str, entity: &str) -> Vec<String> {
    ensure_group_and_entity_dir(group, entity);
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    buf.push(group);
    buf.push(entity);
    let read_dir = std::fs::read_dir(buf).unwrap();
    let mut vec = read_dir
        .filter_map(|x| -> Option<String> {
            if x.is_err() {
                return None;
            }
            let file_name = x.unwrap().file_name().into_string();
            if file_name.is_err() {
                return None;
            }
            let unwrapped_value = file_name.unwrap();
            if unwrapped_value.ends_with(".lua") {
                return Some(unwrapped_value);
            }
            return None;
        })
        .collect::<Vec<String>>();
    vec.sort();
    return vec;
}

fn build_file_name(group: &str, entity: &str, file_name: String) -> PathBuf {
    let mut buf = PathBuf::new();
    buf.push(".");
    buf.push(STORAGE_DIRECTORY);
    buf.push(group);
    buf.push(entity);
    buf.push(file_name);
    buf
}

pub async fn extract_file(
    group: &str,
    entity: &str,
    file_name: String,
) -> std::io::Result<NamedFile> {
    NamedFile::open(build_file_name(group, entity, file_name)).await
}

pub async fn cleanup(group: &str, entity: &str) -> std::io::Result<()> {
    let mut files = collect_files(group, entity);
    let _ = files.split_off(files.len() - 3);
    for file_name in files {
        tokio_fs::remove_file(build_file_name(group, entity, file_name)).await?;
    }
    return Ok(());
}
