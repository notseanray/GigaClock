use crate::model::TokenClaims;
use crate::{
    authenticate_token::AuthenticationGuard,
    google_oauth::{get_google_user, request_token},
    model::{AppState, User},
    CONFIG,
};
use actix_web::cookie::SameSite;
use actix_web::{
    cookie::{time::Duration as ActixWebDuration, Cookie},
    get, post, web, HttpResponse, Responder,
};
use chrono::{prelude::*, Duration as CDuration};
use jsonwebtoken::{encode, EncodingKey, Header};
use reqwest::header::LOCATION;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::SystemTime;
use std::{
    fs::rename,
    time::UNIX_EPOCH,
};

#[derive(Debug, Deserialize)]
pub struct QueryCode {
    pub code: String,
    pub state: String,
}

#[get("/ping")]
async fn ping(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"uptime": data.init_ts.duration_since(UNIX_EPOCH).unwrap().as_nanos().to_string() }))
}

#[post("/upload")]
pub async fn upload(
    parts: awmp::Parts,
    auth_guard: AuthenticationGuard,
    data: web::Data<AppState>,
) -> impl Responder {
    let user = User::get_by_id(&auth_guard.user_id, &data).await;
    if let Ok(mut v) = user {

        // TODO
        let file_name = "TODO";
        let files = parts
            .files
            .into_inner()
            .into_iter()
            .flat_map(|(_name, res_tf)| res_tf.map(|x| (&file_name, x)))
            .map(|(_name, tf)| tf.persist_in("./storage").map(|f| (&file_name, f)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
            .into_iter()
            .map(|(_name, f)| f.display().to_string())
            .collect::<Vec<_>>();
        for file in files {
            rename(file, format!("forms/{file_name}")).unwrap();
        }
        v.lastopen_ts = Some(DateTime::<Utc>::from(SystemTime::now()));
        // let slack_url = env::var("SLACK_URL").unwrap_or_default();
        // if !slack_url.is_empty()
        //     && v.student_member_code_of_conduct
        //     && v.parent_code_of_conduct
        //     && v.club_permission_form
        // {
        //     let client = reqwest::ClientBuilder::new()
        //         .timeout(Duration::from_secs(1))
        //         .build()
        //         .unwrap_or_default();
        //     let _ = client
        //         .post(slack_url)
        //         .header(
        //             HeaderName::from_str("Content-type").unwrap(),
        //             HeaderValue::from_str("application/json").unwrap(),
        //         )
        //         .body(
        //             serde_json::to_string(&json!({ "text": format!(
        //                 "{first} {last} has paid and completed all forms."
        //             )
        //             }))
        //             .unwrap(),
        //         )
        //         .send()
        //         .await;
        // }
        HttpResponse::Ok()
    } else {
        HttpResponse::BadRequest()
    }
}

#[get("/sessions/oauth/google")]
async fn google_oauth_handler(
    query: web::Query<QueryCode>,
    data: web::Data<AppState>,
) -> impl Responder {
    let code = &query.code;
    let state = &query.state;

    if code.is_empty() {
        return HttpResponse::Unauthorized().json(
            serde_json::json!({"status": "fail", "message": "Authorization code not provided!"}),
        );
    }

    let token_response = request_token(code.as_str(), &data).await;
    if token_response.is_err() {
        let message = token_response.err().unwrap().to_string();
        return HttpResponse::BadGateway()
            .json(serde_json::json!({"status": "fail", "message": message}));
    }

    let token_response = token_response.unwrap();
    let google_user = get_google_user(&token_response.access_token, &token_response.id_token).await;
    if google_user.is_err() {
        let message = google_user.err().unwrap().to_string();
        return HttpResponse::BadGateway()
            .json(serde_json::json!({"status": "fail", "message": message}));
    }

    let google_user = google_user;

    let datetime = Utc::now();
    let Ok(user) = google_user else {
        return HttpResponse::BadGateway()
            .json(serde_json::json!({"status": "fail", "message": "Failed to retrieve google user information"}));
    };
    let user_id = user.id.to_owned().to_string();
    let user_data = User {
        admin: CONFIG.admin_emails.contains(&user.email.to_lowercase()),
        id: user.id.to_string(),
        name: user.name,
        verified: user.verified_email,
        email: user.email,
        photo: user.picture,
        lastopen_ts: Some(datetime),
        created_at: datetime,
        updated_at: datetime,
    };
    if user_data.insert(&data).await.is_err() {
        return HttpResponse::BadGateway()
            .json(serde_json::json!({"status": "fail", "message": "Failed to insert to db"}));
    };

    let jwt_secret = data.env.jwt_secret.to_owned();
    let now = Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + CDuration::minutes(data.env.jwt_max_age)).timestamp() as usize;
    let claims: TokenClaims = TokenClaims {
        sub: user_id,
        exp,
        iat,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .unwrap();
    //

    let cookie = Cookie::build("token", token)
        .path("/")
        .same_site(SameSite::None)
        .secure(true)
        .max_age(ActixWebDuration::new(60 * data.env.jwt_max_age, 0))
        .http_only(true)
        .finish();

    let frontend_origin = data.env.client_origin.to_owned();
    let mut response = HttpResponse::Found();
    response.append_header((LOCATION, format!("{}{}", frontend_origin, state)));
    response.cookie(cookie);
    response.finish()
}

#[get("/auth/logout")]
async fn logout_handler(_: AuthenticationGuard) -> impl Responder {
    let cookie = Cookie::build("token", "")
        .path("/")
        .same_site(SameSite::None)
        .secure(true)
        .max_age(ActixWebDuration::new(-1, 0))
        .http_only(true)
        .finish();

    HttpResponse::Ok()
        .cookie(cookie)
        .json(serde_json::json!({"status": "success"}))
}

#[derive(Serialize, Debug)]
pub struct UserResponse {
    pub status: String,
    pub data: User,
}

#[get("/users/me")]
async fn get_me_handler(
    auth_guard: AuthenticationGuard,
    data: web::Data<AppState>,
) -> impl Responder {
    let full_user = User::get_by_id(&auth_guard.user_id, &data).await;
    if let Ok(v) = full_user {
        let json_response = UserResponse {
            status: "success".to_string(),
            data: v.to_owned(),
        };
        return HttpResponse::Ok().json(json_response);
    };
    HttpResponse::Unauthorized().json(json!({ "status": "failed" }))
}

pub fn config(conf: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(ping)
        .service(google_oauth_handler)
        .service(logout_handler)
        .service(upload)
        .service(actix_files::Files::new("/api/alarms", "./storage"))
        .service(get_me_handler);

    conf.service(scope);
}
