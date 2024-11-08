use actix_web::{
    post,
    web::{self},
    App, HttpServer,
};
use serde::{Deserialize, Serialize};
use syncflow_client::ProjectClient;
use syncflow_shared::livekit_models::VideoGrantsWrapper;
use syncflow_shared::project_models::NewSessionRequest;
use syncflow_shared::response_models::Response;
use syncflow_shared::{livekit_models::TokenRequest, utils::load_env};

use actix_files as fs;
use std::env;

use actix_web::HttpResponse;

pub fn json_ok_response<T: serde::Serialize>(data: T) -> HttpResponse {
    HttpResponse::Ok().json(data)
}

pub fn error_response(e: impl Into<Response>) -> HttpResponse {
    e.into().into()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinimalTokenRequest {
    pub identity: String,
    pub room_name: String,
}

impl From<MinimalTokenRequest> for NewSessionRequest {
    fn from(value: MinimalTokenRequest) -> Self {
        NewSessionRequest {
            name: Some(value.room_name),
            auto_recording: Some(true),
            max_participants: Some(200),
            device_groups: Some(vec!["text-egress".to_string()]),
            ..Default::default()
        }
    }
}

#[post("/token")]
async fn get_token(
    client: web::Data<ProjectClient>,
    token_request: web::Json<MinimalTokenRequest>,
) -> actix_web::HttpResponse {
    log::info!("Requesting token, {:#?}", token_request);
    let sessions_result = client.get_sessions().await;
    let token_request = token_request.into_inner();
    let identity = token_request.identity.clone();
    let room_name = token_request.room_name.clone();

    let res = match sessions_result {
        Ok(sessions) => {
            let session = sessions.iter().find(|session| {
                session.name == token_request.room_name && session.status != "Stopped"
            });
            match session {
                Some(session) => {
                    let token = client
                        .generate_session_token(
                            &session.id,
                            &TokenRequest {
                                identity: identity.clone(),
                                name: Some(identity.clone()),
                                video_grants: VideoGrantsWrapper {
                                    can_publish: true,
                                    can_subscribe: true,
                                    room_join: true,
                                    room_create: true,
                                    room: room_name.clone(),
                                    ..Default::default()
                                },
                            },
                        )
                        .await;
                    match token {
                        Ok(token) => json_ok_response(token),
                        Err(_e) => HttpResponse::InternalServerError().finish(),
                    }
                }
                None => {
                    let new_session_req = token_request.into();
                    let new_session = client.create_session(&new_session_req).await;
                    match new_session {
                        Ok(session) => {
                            let token = client
                                .generate_session_token(
                                    &session.id,
                                    &TokenRequest {
                                        identity: identity.clone(),
                                        name: Some(identity.clone()),
                                        video_grants: VideoGrantsWrapper {
                                            can_publish: true,
                                            can_subscribe: true,
                                            room_join: true,
                                            room_create: true,
                                            room: room_name.clone(),
                                            ..Default::default()
                                        },
                                    },
                                )
                                .await;
                            match token {
                                Ok(token) => json_ok_response(token),
                                Err(_e) => HttpResponse::InternalServerError().finish(),
                            }
                        }
                        Err(_e) => HttpResponse::InternalServerError().finish(),
                    }
                }
            }
        }
        Err(_e) => HttpResponse::InternalServerError().finish(),
    };
    log::info!("Token Response {:#?}", res.body());
    res
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    load_env();
    env::set_var(
        "RUST_LOG",
        "actix_web=debug,actix_files=debug,crystall_island_server=debug",
    );
    env_logger::init();
    let project_id = env::var("SYNCFLOW_PROJECT_ID").expect("PROJECT_ID must be set");
    let api_key = env::var("SYNCFLOW_API_KEY").expect("API_KEY must be set");
    let api_secret = env::var("SYNCFLOW_API_SECRET").expect("API_SECRET must be set");
    let base_url = env::var("SYNCFLOW_API_URL").expect("API_URL must be set");

    let client = ProjectClient::new(&base_url, &project_id, &api_key, &api_secret);

    let port = 9000;
    let host = "0.0.0.0";
    let app_data = web::Data::new(client);
    log::info!("Starting server on {}:{}", host, port);
    HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::default())
            .service(get_token)
            .service(
                fs::Files::new("/", "./EngageAI-NLEPrototype")
                    .show_files_listing()
                    .index_file("index.html")
                    .use_last_modified(true),
            )
            .app_data(app_data.clone())
    })
    .workers(4)
    .bind(&format!("{}:{}", host, port))?
    .run()
    .await
}
