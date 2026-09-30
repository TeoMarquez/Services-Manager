#[cfg(test)]
use std::env;
#[cfg(unix)]
use std::{fs::File, io::Read};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;

const TOKEN_KEY: &str = "SERVICES_MANAGER_API_TOKEN";
const PORT_KEY: &str = "SERVICES_MANAGER_API_PORT";
const TOKEN_BYTES: usize = 32;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct TokenOptions {
    pub token: Option<String>,
    pub overwrite: bool,
    pub silent: bool,
    pub port: Option<u16>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfiguredToken {
    pub value: String,
    pub silent: bool,
    pub dotenv_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("invalid API token: {0}")]
    Invalid(String),
    #[error(
        "--token was provided but a different token already exists in {path}; pass --overwrite-token to rotate it"
    )]
    OverwriteRequired { path: PathBuf },
    #[error("invalid command line: {0}")]
    Arguments(String),
    #[error("invalid API port '{0}'; expected a number from 1 to 65535")]
    InvalidPort(String),
    #[error("could not read or write token file: {0}")]
    Io(#[from] io::Error),
}

pub fn parse_args(args: &[String]) -> Result<TokenOptions, TokenError> {
    let mut options = TokenOptions::default();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--token" => {
                index += 1;
                let token = args.get(index).ok_or_else(|| {
                    TokenError::Arguments(
                        "--token requires a value (use \"\" to generate one)".to_owned(),
                    )
                })?;
                options.token = Some(token.clone());
            }
            "--overwrite-token" => options.overwrite = true,
            "--silent-token" => options.silent = true,
            "--port" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    TokenError::Arguments("--port requires a value from 1 to 65535".to_owned())
                })?;
                let port = value
                    .parse::<u16>()
                    .ok()
                    .filter(|port| *port != 0)
                    .ok_or_else(|| TokenError::InvalidPort(value.clone()))?;
                options.port = Some(port);
            }
            "--help" | "-h" => {
                return Err(TokenError::Arguments(
                    "usage: api [--token VALUE] [--overwrite-token] [--silent-token] [--port PORT]; an empty --token generates a token".to_owned(),
                ));
            }
            argument => {
                return Err(TokenError::Arguments(format!(
                    "unknown argument: {argument}"
                )));
            }
        }
        index += 1;
    }
    Ok(options)
}

pub fn configure_port(
    requested_port: Option<u16>,
    environment_port: Option<&str>,
    bind_port: Option<u16>,
    dotenv_path: impl AsRef<Path>,
) -> Result<u16, TokenError> {
    let dotenv_path = dotenv_path.as_ref();
    let saved_port = read_dotenv_value(dotenv_path, PORT_KEY)?;
    let environment_port = environment_port
        .filter(|value| !value.trim().is_empty())
        .map(parse_port)
        .transpose()?;
    let saved_port = saved_port.as_deref().map(parse_port).transpose()?;
    let port = requested_port
        .or(environment_port)
        .or(saved_port)
        .or(bind_port)
        .unwrap_or(3000);
    if requested_port.is_some() || saved_port.is_none() {
        write_dotenv_value(dotenv_path, PORT_KEY, &port.to_string())?;
    }
    Ok(port)
}

fn parse_port(value: &str) -> Result<u16, TokenError> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| TokenError::InvalidPort(value.to_owned()))
}

pub fn configure_token(
    options: TokenOptions,
    environment_token: Option<&str>,
    dotenv_path: impl AsRef<Path>,
) -> Result<ConfiguredToken, TokenError> {
    let dotenv_path = dotenv_path.as_ref().to_path_buf();
    let file_token = read_dotenv_token(&dotenv_path)?;

    let (value, persist) = if options.overwrite {
        let value = match options.token.as_deref() {
            Some(value) if !value.is_empty() => value.to_owned(),
            _ => generate_token()?,
        };
        (value, true)
    } else if let Some(requested) = options.token.as_deref() {
        if requested.is_empty() {
            if file_token.is_some() {
                return Err(TokenError::OverwriteRequired { path: dotenv_path });
            }
            (generate_token()?, true)
        } else if file_token
            .as_deref()
            .is_some_and(|saved| saved != requested)
        {
            return Err(TokenError::OverwriteRequired { path: dotenv_path });
        } else {
            (requested.to_owned(), true)
        }
    } else if let Some(saved) = file_token {
        (saved, false)
    } else if let Some(environment) = environment_token.filter(|token| !token.is_empty()) {
        (environment.to_owned(), true)
    } else {
        (generate_token()?, true)
    };

    validate_token(&value)?;
    if persist {
        write_dotenv_token(&dotenv_path, &value)?;
    }

    Ok(ConfiguredToken {
        value,
        silent: options.silent,
        dotenv_path,
    })
}

fn validate_token(token: &str) -> Result<(), TokenError> {
    if token.len() < TOKEN_BYTES {
        return Err(TokenError::Invalid(format!(
            "must contain at least {TOKEN_BYTES} bytes"
        )));
    }
    if !token.is_ascii()
        || token.bytes().any(|byte| {
            byte.is_ascii_whitespace()
                || byte.is_ascii_control()
                || byte == b'#'
                || byte == b'"'
                || byte == b'\''
        })
    {
        return Err(TokenError::Invalid(
            "must be printable ASCII without whitespace, quotes, or #".to_owned(),
        ));
    }
    Ok(())
}

fn generate_token() -> Result<String, TokenError> {
    let mut bytes = [0_u8; TOKEN_BYTES];
    fill_random(&mut bytes)?;
    let mut token = String::with_capacity(TOKEN_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(token, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(token)
}

#[cfg(unix)]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    File::open("/dev/urandom")?.read_exact(output)
}

#[cfg(windows)]
fn fill_random(output: &mut [u8]) -> io::Result<()> {
    use std::ffi::c_void;
    #[link(name = "bcrypt")]
    unsafe extern "system" {
        fn BCryptGenRandom(algorithm: *mut c_void, buffer: *mut u8, length: u32, flags: u32)
        -> i32;
    }
    const BCRYPT_USE_SYSTEM_PREFERRED_RNG: u32 = 0x00000002;
    let status = unsafe {
        BCryptGenRandom(
            std::ptr::null_mut(),
            output.as_mut_ptr(),
            output.len() as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG,
        )
    };
    if status < 0 {
        return Err(io::Error::from_raw_os_error(status));
    }
    Ok(())
}

#[cfg(not(any(unix, windows)))]
fn fill_random(_: &mut [u8]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "secure token generation is unsupported on this platform",
    ))
}

fn read_dotenv_token(path: &Path) -> io::Result<Option<String>> {
    read_dotenv_value(path, TOKEN_KEY)
}

fn read_dotenv_value(path: &Path, requested_key: &str) -> io::Result<Option<String>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    Ok(content.lines().find_map(|line| {
        let line = line.trim();
        if line.starts_with('#') {
            return None;
        }
        let (key, value) = line.split_once('=')?;
        if key.trim() != requested_key {
            return None;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|quoted| quoted.strip_suffix('"'))
            .or_else(|| {
                value
                    .strip_prefix('\'')
                    .and_then(|quoted| quoted.strip_suffix('\''))
            })
            .unwrap_or(value);
        (!value.is_empty()).then(|| value.to_owned())
    }))
}

fn write_dotenv_token(path: &Path, token: &str) -> io::Result<()> {
    write_dotenv_value(path, TOKEN_KEY, token)
}

fn write_dotenv_value(path: &Path, requested_key: &str, value: &str) -> io::Result<()> {
    let mut lines = match fs::read_to_string(path) {
        Ok(contents) => contents.lines().map(str::to_owned).collect::<Vec<_>>(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error),
    };
    let mut replaced = false;
    lines.retain_mut(|line| {
        let is_token_line = line
            .trim()
            .split_once('=')
            .is_some_and(|(key, _)| key.trim() == requested_key);
        if is_token_line {
            if replaced {
                return false;
            }
            *line = format!("{requested_key}={value}");
            replaced = true;
        }
        true
    });
    if !replaced {
        lines.push(format!("{requested_key}={value}"));
    }
    let mut content = lines.join("\n");
    content.push('\n');

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        if path.exists() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }
    let mut file = options.open(path)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_path() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        env::temp_dir().join(format!("services-manager-env-{}-{id}", std::process::id()))
    }

    #[test]
    fn generates_and_persists_a_secure_length_token_when_file_is_missing() {
        let path = test_path();
        let configured = configure_token(TokenOptions::default(), None, &path).unwrap();
        assert_eq!(configured.value.len(), 64);
        assert_eq!(
            read_dotenv_token(&path).unwrap().as_deref(),
            Some(configured.value.as_str())
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn saved_token_wins_over_environment_until_explicit_overwrite() {
        let path = test_path();
        write_dotenv_token(&path, "a".repeat(32).as_str()).unwrap();
        let configured =
            configure_token(TokenOptions::default(), Some(&"b".repeat(32)), &path).unwrap();
        assert_eq!(configured.value, "a".repeat(32));

        let options = TokenOptions {
            token: Some("c".repeat(32)),
            overwrite: true,
            silent: true,
            port: None,
        };
        let configured = configure_token(options, None, &path).unwrap();
        assert_eq!(configured.value, "c".repeat(32));
        assert!(configured.silent);
        assert_eq!(
            read_dotenv_token(&path).unwrap().as_deref(),
            Some("c".repeat(32).as_str())
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn explicit_different_token_requires_overwrite_flag() {
        let path = test_path();
        write_dotenv_token(&path, "a".repeat(32).as_str()).unwrap();
        let options = TokenOptions {
            token: Some("b".repeat(32)),
            overwrite: false,
            silent: false,
            port: None,
        };
        assert!(matches!(
            configure_token(options, None, &path),
            Err(TokenError::OverwriteRequired { .. })
        ));
        fs::remove_file(path).unwrap();
    }
}
