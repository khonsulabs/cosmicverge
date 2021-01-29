use std::str::FromStr;

use jsonwebtoken::{DecodingKey, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtKey {
    #[serde(rename = "alg")]
    pub algorithm: String,
    #[serde(rename = "kid")]
    pub key_id: String,
    #[serde(rename = "kty")]
    pub key_type: String,
    #[serde(rename = "e")]
    pub rsa_e: String,
    #[serde(rename = "n")]
    pub rsa_n: String,
    #[serde(rename = "use")]
    pub public_use: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("only rsa keys are supported currently")]
    NonRSAKey,
    #[error("jwt error: {0}")]
    JsonWebToken(#[from] jsonwebtoken::errors::Error),
}

impl JwtKey {
    pub fn parse_token<T>(&self, token: &str) -> Result<jsonwebtoken::TokenData<T>, Error>
        where
                for<'de> T: Deserialize<'de>,
    {
        if self.key_type != "RSA" {
            return Err(Error::NonRSAKey);
        }

        let algorithm = jsonwebtoken::Algorithm::from_str(&self.algorithm)?;
        let jwt_key = DecodingKey::from_rsa_components(&self.rsa_n, &self.rsa_e);

        let validation = Validation::new(algorithm);
        Ok(jsonwebtoken::decode::<T>(token, &jwt_key, &validation)?)
    }
}
