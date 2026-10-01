//! Validation des jetons émis côté serveur, indépendamment de l'identité Riot.
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub aud: String,
    pub exp: u64,
    pub nbf: u64,
}

pub struct Auth {
    encoding: EncodingKey,
    decoding: DecodingKey,
    validation: Validation,
    issuer: String,
    audience: String,
}
impl Auth {
    /// Construit un validateur à algorithme fixe ; aucune valeur sensible dans les erreurs.
    pub fn new(secret: &[u8], issuer: &str, audience: &str) -> Result<Self, &'static str> {
        if secret.len() < 32 || issuer.trim().is_empty() || audience.trim().is_empty() {
            return Err("configuration JWT invalide");
        }
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 0;
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "nbf", "iss", "aud", "sub"]);
        validation.set_issuer(&[issuer]);
        validation.set_audience(&[audience]);
        Ok(Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            validation,
            issuer: issuer.into(),
            audience: audience.into(),
        })
    }
    /// Émission réservée à la commande serveur ; aucun endpoint public ne signe de jeton.
    pub fn issue(&self, subject: &str, now: u64, ttl: u64) -> Result<String, &'static str> {
        if subject.trim().is_empty() || subject.len() > 128 || !(1..=86400).contains(&ttl) {
            return Err("paramètres du jeton invalides");
        }
        let exp = now
            .checked_add(ttl)
            .ok_or("paramètres du jeton invalides")?;
        encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                sub: subject.into(),
                iss: self.issuer.clone(),
                aud: self.audience.clone(),
                exp,
                nbf: now,
            },
            &self.encoding,
        )
        .map_err(|_| "émission impossible")
    }
    /// Valide signature, dates, audience et émetteur avant de lire le sujet.
    pub fn verify(&self, token: &str) -> Result<Claims, &'static str> {
        if token.len() > 4096 {
            return Err("unauthorized");
        }
        let claims = decode::<Claims>(token, &self.decoding, &self.validation)
            .map_err(|_| "unauthorized")?
            .claims;
        if claims.sub.trim().is_empty()
            || claims.sub.len() > 128
            || claims.exp <= jsonwebtoken::get_current_timestamp()
            || claims.exp <= claims.nbf
        {
            return Err("unauthorized");
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::get_current_timestamp;

    // Clé synthétique de test, sans relation avec un compte ou un déploiement.
    const TEST_KEY: &[u8] = b"synthetic-signing-material-for-tests-only";

    #[test]
    fn accepte_uniquement_un_jeton_valide_pour_cette_api() {
        let auth = Auth::new(TEST_KEY, "test-issuer", "test-api").unwrap();
        let token = auth
            .issue("test-session", get_current_timestamp(), 60)
            .unwrap();
        assert_eq!(auth.verify(&token).unwrap().sub, "test-session");
        assert!(Auth::new(TEST_KEY, "other", "test-api")
            .unwrap()
            .verify(&token)
            .is_err());
        assert!(Auth::new(TEST_KEY, "test-issuer", "other")
            .unwrap()
            .verify(&token)
            .is_err());
        assert!(Auth::new(
            b"different-synthetic-signing-material",
            "test-issuer",
            "test-api"
        )
        .unwrap()
        .verify(&token)
        .is_err());
    }

    #[test]
    fn refuse_expiration_future_activation_alteration_et_configuration_faible() {
        let auth = Auth::new(TEST_KEY, "issuer", "api").unwrap();
        let now = get_current_timestamp();
        assert!(auth
            .verify(&auth.issue("session", now - 100, 1).unwrap())
            .is_err());
        assert!(auth
            .verify(&auth.issue("session", now + 100, 60).unwrap())
            .is_err());
        let token = auth.issue("session", now, 60).unwrap();
        assert!(auth.verify(&format!("{token}changed")).is_err());
        assert!(auth.verify("not-a-token").is_err());
        assert!(auth.issue("", now, 60).is_err());
        assert!(Auth::new(b"short", "issuer", "api").is_err());
        assert!(Auth::new(TEST_KEY, "", "api").is_err());
    }
}
