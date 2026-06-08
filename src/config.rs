use serde::Deserialize;
use std::io;

/// Legacy config format (stocktake.toml) — flat key=value style.
#[derive(Deserialize)]
struct LegacyConfig {
    listen_addr: Option<String>,
    port: Option<u16>,
    connection_string: Option<String>,
}

/// New config format (config.toml) — sectioned style.
#[derive(Deserialize)]
struct NewConfig {
    server: Option<ServerConfig>,
    database: Option<DbConfig>,
}

#[derive(Deserialize, Default)]
struct ServerConfig {
    host: Option<String>,
    port: Option<u16>,
    output_dir: Option<String>,
}

#[derive(Deserialize, Default)]
struct DbConfig {
    connection_string: Option<String>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub connection_string: String,
    pub output_dir: String,
}

/// Try loading `config.toml` (new format) first, then fall back to
/// `stocktake.toml` (legacy flat format). Returns a default config if neither
/// file exists.
///
/// IMPORTANT: a file that EXISTS but fails to parse is a hard error — we return
/// it to the caller instead of silently falling through to defaults. Previously
/// a malformed `config.toml` (e.g. an unescaped backslash in a Windows path or
/// password, which TOML treats as an invalid string escape) was swallowed and
/// reported misleadingly as "no config file found". Surfacing the real parse
/// error — with its line/column — is the single most useful diagnostic.
pub fn load() -> Result<AppConfig, io::Error> {
    let new_exists = std::path::Path::new("config.toml").exists();
    let legacy_exists = std::path::Path::new("stocktake.toml").exists();

    // New format takes precedence when present.
    if new_exists {
        return try_load_new();
    }
    if legacy_exists {
        return try_load_legacy();
    }

    // Neither file is on disk — fall back to defaults (empty connection string,
    // which main() will reject with a clear "set connection_string" message).
    println!("⚠ No config file found. Using defaults.");
    println!("   Please create config.toml or stocktake.toml in the same folder as this program.");
    Ok(AppConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        connection_string: String::new(),
        output_dir: "stocktake_output".to_string(),
    })
}

fn try_load_new() -> Result<AppConfig, io::Error> {
    let path = "config.toml";
    let raw = std::fs::read_to_string(path)?;
    let cfg: NewConfig = toml::from_str(&raw).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "config.toml exists but could not be parsed: {}\n   \
                 Common cause: a backslash in a Windows path or password. In TOML, \\\n   \
                 inside \"...\" is an escape character — write paths as C:\\\\StockTakes \\\n   \
                 (doubled backslashes) or use single quotes: 'C:\\StockTakes'.",
                e
            ),
        )
    })?;

    let server = cfg.server.unwrap_or_default();
    let db = cfg.database.unwrap_or_default();

    println!("📄 Loaded config from: {}", path);
    Ok(AppConfig {
        host: server.host.unwrap_or_else(|| "127.0.0.1".to_string()),
        port: server.port.unwrap_or(8080),
        connection_string: db.connection_string.unwrap_or_default(),
        output_dir: server.output_dir.unwrap_or_else(|| "stocktake_output".to_string()),
    })
}

fn try_load_legacy() -> Result<AppConfig, io::Error> {
    let path = "stocktake.toml";
    let raw = std::fs::read_to_string(path)?;
    let cfg: LegacyConfig = toml::from_str(&raw).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "stocktake.toml exists but could not be parsed: {}\n   \
                 Common cause: a backslash in a Windows path or password. In TOML, \\\n   \
                 inside \"...\" is an escape character — double it (\\\\) or use single quotes.",
                e
            ),
        )
    })?;

    println!("📄 Loaded config from: {}", path);
    println!("   ⚠ Using legacy config format — consider migrating to config.toml");
    Ok(AppConfig {
        host: cfg.listen_addr.unwrap_or_else(|| "127.0.0.1".to_string()),
        port: cfg.port.unwrap_or(8080),
        connection_string: cfg.connection_string.unwrap_or_default(),
        output_dir: "stocktake_output".to_string(),
    })
}


