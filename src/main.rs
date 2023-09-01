#[macro_use]
extern crate rocket;
use rocket::config::Config;
use rocket::data::{Limits, ToByteUnit};
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::status::NotFound;
use rocket::serde::{json::Json, Serialize};
use rocket::tokio::io::AsyncReadExt;
use rocket::Data;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

mod storage;

struct ApiKey<'r>(&'r str);

#[derive(Debug)]
enum ApiKeyError {
    Missing,
    Invalid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey<'r> {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        /// Returns true if `key` is a valid API key string.
        fn is_valid(key: &str) -> bool {
            key == "valid_api_key"
        }

        match req.headers().get_one("x-api-key") {
            None => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            Some(key) if is_valid(key) => Outcome::Success(ApiKey(key)),
            Some(_) => Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid)),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Files {
    files: Vec<String>,
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Info {
    latest: String,
}

#[post("/storage/<group>/<entity>", data = "<data>")]
async fn post_file(
    group: &str,
    entity: &str,
    data: Data<'_>,
    _key: ApiKey<'_>,
) -> std::io::Result<()> {
    let files = storage::collect_files(group, entity);
    let newest_file = files.last();
    let file_name = storage::generate_file_name(group, entity);
    if newest_file.is_none() {
        data.open(1.mebibytes()).into_file(file_name).await?;
    } else {
        let newest_file_name: String = files.last().unwrap().to_string();
        let mut file_data = storage::extract_file(group, entity, newest_file_name).await?;
        let mut old_data: Vec<u8> = Vec::new();
        file_data.read_to_end(&mut old_data).await?;
        let new_data = data.open(1.mebibytes()).into_bytes().await?;
        if !new_data.is_complete() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Something is not quite right",
            ));
        }

        if old_data == new_data.value {
            return Ok(());
        }
        println!(
            "Files are not equivalent (?), {}, {}",
            old_data.last().unwrap(),
            new_data.value.last().unwrap()
        );
        let mut new_file = File::create(file_name).await?;
        let mut buffer = tokio::io::BufWriter::new(&mut new_file);
        buffer.write(&new_data.value).await?;
        storage::cleanup(group, entity).await?;
    }
    Ok(())
}

#[get("/storage/<group>/<entity>/versions")]
async fn get_versions(group: &str, entity: &str) -> Json<Files> {
    return Json(Files {
        files: storage::collect_files(group, entity),
    });
}

#[get("/storage/<group>/<entity>/info")]
async fn get_info(group: &str, entity: &str) -> Json<Info> {
    let files = storage::collect_files(group, entity);
    let newest_file: String = files
        .last()
        .unwrap_or(&String::from(storage::DEFAULT_FILE))
        .to_string();
    return Json(Info {
        latest: newest_file.replace(".lua", ""),
    });
}

#[get("/storage/<group>/<entity>/latest")]
async fn get_latest(group: &str, entity: &str) -> Result<NamedFile, NotFound<String>> {
    let files = storage::collect_files(group, entity);
    let newest_file = files.last();
    if newest_file.is_none() {
        return Err(NotFound("there is no versions for this file".to_string()));
    }
    let newest_file_name: String = files.last().unwrap().to_string();
    return storage::extract_file(group, entity, newest_file_name)
        .await
        .map_err(|e| NotFound(e.to_string()));
}

#[launch]
fn rocket() -> _ {
    storage::ensure_dir();
    let mut config = Config::default();
    config.limits = Limits::default().limit("form", 1.mebibytes());
    rocket::build()
        .configure(config)
        .mount("/", routes![post_file, get_versions, get_info, get_latest])
}
