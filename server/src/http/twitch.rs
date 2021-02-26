use std::convert::Infallible;

use chrono::{NaiveDateTime, Utc};
use database::{
    basws_server::prelude::Uuid,
    pool,
    schema::{Account, Installation, OAuthToken, TwitchProfile},
};
use serde::{Deserialize, Serialize};
use url::Url;
use warp::{Filter, Rejection};

use crate::http::{jwk::JwtKey, webserver_base_url};

#[derive(Deserialize)]
struct TwitchCallback {
    code: String,
    state: String,
    // scope: String,
}

pub fn callback_uri() -> String {
    webserver_base_url()
        .path_and_query("/v1/auth/callback/twitch")
        .build()
        .unwrap()
        .to_string()
}

pub fn callback() -> impl warp::Filter<Extract = (impl warp::Reply,), Error = Rejection> + Copy {
    warp::path!("auth" / "callback" / "twitch")
        .and(warp::query())
        .and_then(|callback: TwitchCallback| async move { callback.respond().await })
}

impl TwitchCallback {
    async fn respond(self) -> Result<impl warp::Reply, Infallible> {
        // TODO bad unwrap
        login_twitch(self.state.parse().unwrap(), self.code)
            .await
            .unwrap();

        Ok(warp::redirect::redirect(
            webserver_base_url().path_and_query("/").build().unwrap(),
        ))
    }
}

pub fn authorization_url(installation_id: Uuid) -> String {
    Url::parse_with_params(
        "https://id.twitch.tv/oauth2/authorize",
        &[
            ("client_id", std::env::var("TWITCH_CLIENT_ID").unwrap()),
            ("scope", "openid".to_owned()),
            ("response_type", "code".to_owned()),
            ("redirect_uri", callback_uri()),
            ("state", installation_id.to_string()),
            // TODO add NONCE
        ],
    )
    .unwrap()
    .to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct TwitchTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<usize>,
    pub scope: Vec<String>,
    pub id_token: String,
    pub token_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct TwitchUserInfo {
    pub id: String,
    pub login: String,
    pub display_name: Option<String>,
    // pub type: Option<String>,
    // pub broadcaster_type: String,
    // pub description: Option<String>,
    // pub profile_image_url: Option<String>,
    // pub offline_image_url: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
struct TwitchUsersResponse {
    pub data: Vec<TwitchUserInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtKeys {
    pub keys: Vec<JwtKey>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    #[serde(rename = "iss")]
    pub issuer: Option<String>,
    #[serde(rename = "sub")]
    pub subject: Option<String>,
    #[serde(rename = "aud")]
    pub audience: Option<String>,
    #[serde(rename = "exp")]
    pub expiration_time: Option<u64>,
    #[serde(rename = "iat")]
    pub issuance_time: Option<u64>,
}

pub async fn login_twitch(installation_id: Uuid, code: String) -> Result<(), anyhow::Error> {
    // Call twitch.tv API to get the user information
    let client = reqwest::Client::new();
    let tokens: TwitchTokenResponse = client
        .post("https://id.twitch.tv/oauth2/token")
        .query(&[
            ("code", code),
            ("client_id", std::env::var("TWITCH_CLIENT_ID").unwrap()),
            (
                "client_secret",
                std::env::var("TWITCH_CLIENT_SECRET").unwrap(),
            ),
            ("grant_type", "authorization_code".to_owned()),
            ("redirect_uri", callback_uri()),
        ])
        .send()
        .await?
        .json()
        .await?;

    let jwt_keys: JwtKeys = client
        .get("https://id.twitch.tv/oauth2/keys")
        .send()
        .await?
        .json()
        .await?;
    let jwt_key = jwt_keys
        .keys
        .into_iter()
        .find(|key| key.key_type == "RSA")
        .expect("Twitch has no RS256 keys");
    let token = jwt_key.parse_token::<JwtClaims>(&tokens.id_token)?;

    let expiration_time = NaiveDateTime::from_timestamp(
        token
            .claims
            .expiration_time
            .ok_or_else(|| anyhow::anyhow!("jwt missing expiration"))? as i64,
        0,
    );
    if token.claims.issuer != Some("https://id.twitch.tv/oauth2".to_owned())
        || expiration_time < Utc::now().naive_utc()
    {
        anyhow::bail!("Invalid JWT Token");
    }
    // TODO check nonce once added

    let response: TwitchUsersResponse = client
        .get("https://api.twitch.tv/helix/users")
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", tokens.access_token),
        )
        .header("client-id", std::env::var("TWITCH_CLIENT_ID").unwrap())
        .send()
        .await?
        .json()
        .await?;

    let user = response
        .data
        .first()
        .ok_or_else(|| anyhow::anyhow!("Expected a user response, but got no users"))?;

    let display_name = user
        .display_name
        .clone()
        .unwrap_or_else(|| user.login.clone());
    let pg = pool();
    {
        let mut tx = pg.begin().await?;

        // Create an account if it doesn't exist yet for this installation
        let account = if let Some(account) =
            Account::find_by_installation_id(installation_id, &mut tx).await?
        {
            account
        } else {
            let account =
                if let Some(account) = Account::find_by_twitch_id(&user.id, &mut tx).await? {
                    account
                } else {
                    Account::create(&mut tx).await?
                };
            Installation::set_account_id_for_installation_id(
                installation_id,
                Some(account.id),
                &mut tx,
            )
            .await?;
            account
        };

        TwitchProfile::associate(&user.id, account.id, &display_name, &mut tx).await?;

        OAuthToken::update(
            account.id,
            "twitch",
            &tokens.access_token,
            tokens.refresh_token.as_deref(),
            &mut tx,
        )
        .await?;

        tx.commit().await?;
    }

    crate::pubsub::notify("installation_login", installation_id.to_string()).await?;

    Ok(())
}
