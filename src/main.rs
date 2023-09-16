#[macro_use]
extern crate rocket;
use rocket::data::{Limits, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::fs::NamedFile;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::status::NotFound;
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::tokio::io::AsyncReadExt;
use rocket::Data;
use rocket::State;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

use rocket::figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment, Profile,
};

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
        let valid_key_outcome = req
            .guard::<&State<MyConfig>>()
            .await
            .map(|my_config| my_config.api_key.clone());

        if !valid_key_outcome.is_success() {
            return Outcome::Failure((Status::InternalServerError, ApiKeyError::Missing));
        }
        let binding = valid_key_outcome.unwrap();
        let valid_key = binding.as_str();

        match req.headers().get_one("x-api-key") {
            None => Outcome::Failure((Status::BadRequest, ApiKeyError::Missing)),
            Some(key) if key == valid_key => Outcome::Success(ApiKey(key)),
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
) -> std::io::Result<String> {
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
            return Ok("File is a duplication and will be ignored".to_string());
        }
        let mut new_file = File::create(file_name.clone()).await?;
        let mut buffer = tokio::io::BufWriter::new(&mut new_file);
        println!("Writing {} to file {}", new_data.value.len(), file_name);
        buffer.write(&new_data.value).await?;
        buffer.shutdown().await?;
        storage::cleanup(group, entity).await?;
    }
    Ok("File uploaded successfully".to_string())
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

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
struct MyConfig {
    api_key: String,
}

impl Default for MyConfig {
    fn default() -> MyConfig {
        MyConfig {
            api_key: "test".into(),
        }
    }
}

#[launch]
fn rocket() -> _ {
    storage::ensure_dir();
    let figment = Figment::from(rocket::Config::default())
        .merge(Serialized::defaults(MyConfig::default()))
        .merge(Toml::file("App.toml").nested())
        .merge(Env::prefixed("APP_").global())
        .merge(("limits", Limits::default().limit("form", 1.mebibytes())))
        .select(Profile::from_env_or("APP_PROFILE", "default"));

    rocket::custom(figment)
        .mount("/", routes![post_file, get_versions, get_info, get_latest])
        .attach(AdHoc::config::<MyConfig>())
}
