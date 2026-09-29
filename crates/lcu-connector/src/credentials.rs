use base64::{engine::general_purpose::STANDARD, Engine as _};
use thiserror::Error;

/// Identifiants de la League Client API, valables tant que le client tourne.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub pid: u32,
    pub port: u16,
    pub password: String,
    pub protocol: String,
}

// Le mot de passe ne doit jamais apparaître dans les logs.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("pid", &self.pid)
            .field("port", &self.port)
            .field("password", &"***")
            .field("protocol", &self.protocol)
            .finish()
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("lockfile vide")]
    Empty,
    #[error("lockfile mal formé : {0} champs au lieu de 5")]
    FieldCount(usize),
    #[error("champ `{field}` invalide : {value}")]
    InvalidField { field: &'static str, value: String },
    #[error("arguments du processus incomplets : `{0}` introuvable")]
    MissingArg(&'static str),
}

impl Credentials {
    /// Lit le contenu d'un lockfile : `LeagueClient:12345:54321:motdepasse:https`.
    pub fn from_lockfile(content: &str) -> Result<Self, ParseError> {
        let content = content.trim();
        if content.is_empty() {
            return Err(ParseError::Empty);
        }
        let parts: Vec<&str> = content.split(':').collect();
        if parts.len() != 5 {
            return Err(ParseError::FieldCount(parts.len()));
        }
        Ok(Self {
            pid: parse_num("pid", parts[1])?,
            port: parse_num("port", parts[2])?,
            password: non_empty("password", parts[3])?,
            protocol: non_empty("protocol", parts[4])?,
        })
    }

    /// Secours quand le lockfile est introuvable : lit la ligne de commande de
    /// `LeagueClientUx` (`--app-port=… --remoting-auth-token=… --app-pid=…`).
    pub fn from_process_args(cmdline: &str) -> Result<Self, ParseError> {
        let port = arg_value(cmdline, "--app-port=").ok_or(ParseError::MissingArg("--app-port"))?;
        let token = arg_value(cmdline, "--remoting-auth-token=")
            .ok_or(ParseError::MissingArg("--remoting-auth-token"))?;
        let pid = arg_value(cmdline, "--app-pid=").unwrap_or("0");
        Ok(Self {
            pid: parse_num("pid", pid)?,
            port: parse_num("port", port)?,
            password: non_empty("password", token)?,
            protocol: "https".into(),
        })
    }

    /// URL de base de l'API locale, ex. `https://127.0.0.1:54321`.
    pub fn base_url(&self) -> String {
        format!("{}://127.0.0.1:{}", self.protocol, self.port)
    }

    /// URL du WebSocket (protocole WAMP) pour s'abonner aux événements.
    pub fn websocket_url(&self) -> String {
        let scheme = if self.protocol == "https" {
            "wss"
        } else {
            "ws"
        };
        format!("{scheme}://127.0.0.1:{}", self.port)
    }

    /// Valeur de l'en-tête `Authorization` : Basic base64("riot:<mot de passe>").
    pub fn authorization_header(&self) -> String {
        format!(
            "Basic {}",
            STANDARD.encode(format!("riot:{}", self.password))
        )
    }
}

fn parse_num<T: std::str::FromStr>(field: &'static str, value: &str) -> Result<T, ParseError> {
    value.parse().map_err(|_| ParseError::InvalidField {
        field,
        value: value.to_string(),
    })
}

fn non_empty(field: &'static str, value: &str) -> Result<String, ParseError> {
    if value.is_empty() {
        Err(ParseError::InvalidField {
            field,
            value: String::new(),
        })
    } else {
        Ok(value.to_string())
    }
}

/// Valeur d'un argument `--nom=valeur`, avec ou sans guillemets autour de l'argument.
fn arg_value<'a>(cmdline: &'a str, key: &str) -> Option<&'a str> {
    let start = cmdline.find(key)? + key.len();
    let rest = &cmdline[start..];
    let end = rest
        .find(|c: char| c == '"' || c.is_whitespace())
        .unwrap_or(rest.len());
    Some(&rest[..end]).filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_un_lockfile_valide() {
        let c = Credentials::from_lockfile("LeagueClient:21340:58703:Zq8_aB-x:https\n").unwrap();
        assert_eq!(c.pid, 21340);
        assert_eq!(c.port, 58703);
        assert_eq!(c.password, "Zq8_aB-x");
        assert_eq!(c.base_url(), "https://127.0.0.1:58703");
        assert_eq!(c.websocket_url(), "wss://127.0.0.1:58703");
    }

    #[test]
    fn en_tete_basic_riot() {
        let c = Credentials::from_lockfile("LeagueClient:1:2:secret:https").unwrap();
        // base64("riot:secret")
        assert_eq!(c.authorization_header(), "Basic cmlvdDpzZWNyZXQ=");
    }

    #[test]
    fn refuse_les_lockfiles_invalides() {
        assert_eq!(Credentials::from_lockfile("  "), Err(ParseError::Empty));
        assert_eq!(
            Credentials::from_lockfile("a:b:c"),
            Err(ParseError::FieldCount(3))
        );
        assert!(matches!(
            Credentials::from_lockfile("LeagueClient:1:notaport:pw:https"),
            Err(ParseError::InvalidField { field: "port", .. })
        ));
        assert!(matches!(
            Credentials::from_lockfile("LeagueClient:1:2::https"),
            Err(ParseError::InvalidField {
                field: "password",
                ..
            })
        ));
    }

    #[test]
    fn lit_les_arguments_windows_entre_guillemets() {
        let cmd = r#""C:/Riot Games/League of Legends/LeagueClientUx.exe" "--riotclient-auth-token=x" "--app-port=61234" "--remoting-auth-token=tok_EN" "--app-pid=4242""#;
        let c = Credentials::from_process_args(cmd).unwrap();
        assert_eq!(
            (c.port, c.pid, c.password.as_str()),
            (61234, 4242, "tok_EN")
        );
    }

    #[test]
    fn lit_les_arguments_macos() {
        let cmd = "/Applications/League of Legends.app/Contents/LoL/LeagueClient.app/Contents/Frameworks/LeagueClientUx.app/Contents/MacOS/LeagueClientUx --app-port=50001 --remoting-auth-token=abc123";
        let c = Credentials::from_process_args(cmd).unwrap();
        assert_eq!((c.port, c.pid, c.password.as_str()), (50001, 0, "abc123"));
    }

    #[test]
    fn signale_l_argument_manquant() {
        assert_eq!(
            Credentials::from_process_args("LeagueClientUx --app-port=1"),
            Err(ParseError::MissingArg("--remoting-auth-token"))
        );
    }

    #[test]
    fn le_debug_masque_le_mot_de_passe() {
        let c = Credentials::from_lockfile("LeagueClient:1:2:secret:https").unwrap();
        assert!(!format!("{c:?}").contains("secret"));
    }
}
