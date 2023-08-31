#[macro_use] extern crate rocket;
use rocket::http::Status;
use rocket::request::{Outcome, Request, FromRequest};
use rocket::fs::TempFile;
use rocket::data::{Limits, ToByteUnit};
use rocket::config::Config;
use rocket::Data;
use rocket::serde::{Serialize, json::Json};

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

#[derive(FromForm)]
struct Upload<'f> {
    file: TempFile<'f>
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct Files {
  files: Vec<String>,
}


#[post("/storage/<group>/<entity>", data = "<data>")]
async fn post_file(group: &str, entity: &str, data: Data<'_>, _key: ApiKey<'_>) -> std::io::Result<()>{
    data.open(1.mebibytes()).into_file(storage::generate_file_name(group, entity)).await?;
    Ok(())
}

#[get("/storage/<group>/<entity>/versions")]
async fn get_versions(group: &str, entity: &str, _key: ApiKey<'_>) -> Json<Files>{
    return Json(Files{ files: storage::collect_files(group, entity) })
} 

// #[get("/storage/<group>/<file>/info")]
// fn index() -> &'static str {
//     "Hello, world!"
// }

#[launch]
fn rocket() -> _ {
    storage::ensure_dir();
    let mut config = Config::default();
    config.limits = Limits::default().limit("form", 1.mebibytes());
    rocket::build().configure(config).mount("/", routes![post_file, get_versions])
}
