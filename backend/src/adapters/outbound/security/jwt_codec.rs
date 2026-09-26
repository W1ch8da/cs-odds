use anyhow::anyhow;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::{
    application::{
        error::{AppError, AppResult},
        ports::outbound::{AccessTokenCodec, IssuedToken},
        principal::Principal,
    },
    domain::user::{User, UserId},
};

const ISSUER: &str = "cs-odds";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: uuid::Uuid,
    role: String,
    iss: String,
    iat: i64,
    exp: i64,
}

/// HS256 JWTs signed with a server secret.
pub struct JwtCodec {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    ttl: Duration,
}

impl JwtCodec {
    pub fn new(secret: &[u8], ttl: Duration) -> Self {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[ISSUER]);
        validation.set_required_spec_claims(&["exp", "sub", "iss"]);
        validation.leeway = 5;
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation,
            ttl,
        }
    }
}

impl AccessTokenCodec for JwtCodec {
    fn issue(&self, user: &User, now: DateTime<Utc>) -> AppResult<IssuedToken> {
        let expires_at = now + self.ttl;
        let claims = Claims {
            sub: user.id.0,
            role: user.role.as_str().to_string(),
            iss: ISSUER.to_string(),
            iat: now.timestamp(),
            exp: expires_at.timestamp(),
        };
        let token = encode(&Header::new(Algorithm::HS256), &claims, &self.encoding)
            .map_err(|e| anyhow!("failed to sign access token: {e}"))?;
        Ok(IssuedToken { token, expires_at })
    }

    fn verify(&self, token: &str) -> AppResult<Principal> {
        let data = decode::<Claims>(token, &self.decoding, &self.validation).map_err(|_| AppError::Unauthorized)?;
        Ok(Principal {
            user_id: UserId(data.claims.sub),
            role: data.claims.role.parse().map_err(|_| AppError::Unauthorized)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::user::{Email, PersonName, Role};

    fn user() -> User {
        User {
            id: UserId(uuid::Uuid::new_v4()),
            email: Email::parse("maya@cs-odds.example").unwrap(),
            name: PersonName::parse("Maya Chen").unwrap(),
            role: Role::Agent,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn round_trip() {
        let codec = JwtCodec::new(b"test-secret-that-is-long-enough-123", Duration::minutes(15));
        let u = user();
        let issued = codec.issue(&u, Utc::now()).unwrap();
        let principal = codec.verify(&issued.token).unwrap();
        assert_eq!(principal, Principal { user_id: u.id, role: Role::Agent });
    }

    #[test]
    fn rejects_expired_tampered_and_foreign_tokens() {
        let codec = JwtCodec::new(b"test-secret-that-is-long-enough-123", Duration::minutes(15));
        let expired = codec.issue(&user(), Utc::now() - Duration::hours(1)).unwrap();
        assert!(matches!(codec.verify(&expired.token), Err(AppError::Unauthorized)));

        let other = JwtCodec::new(b"a-different-secret-also-long-enough", Duration::minutes(15));
        let foreign = other.issue(&user(), Utc::now()).unwrap();
        assert!(matches!(codec.verify(&foreign.token), Err(AppError::Unauthorized)));

        let mut tampered = codec.issue(&user(), Utc::now()).unwrap().token;
        tampered.push('x');
        assert!(matches!(codec.verify(&tampered), Err(AppError::Unauthorized)));
    }
}
