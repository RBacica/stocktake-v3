mod config;
mod db;
mod server;

use actix_files::Files;
use actix_web::{middleware::Logger, web, App, HttpServer};
use std::io::Write;

#[actix_web::main]
async fn main() {
    // Run the real program. On ANY startup error, print it clearly and pause so
    // the console window stays open long enough to read the message — otherwise
    // double-clicking the .exe just flashes a window that closes instantly.
    if let Err(e) = run().await {
        eprintln!();
        eprintln!("❌ Stocktake failed to start:");
        eprintln!("   {}", e);
        eprintln!();
        pause_before_exit();
        std::process::exit(1);
    }
}

/// Block until the user presses Enter, so error messages remain visible when
/// the program is launched by double-clicking on Windows.
fn pause_before_exit() {
    print!("Press Enter to close this window... ");
    let _ = std::io::stdout().flush();
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
}

/// Ensure the process's working directory is the folder that contains the
/// executable. Without this, `config.toml` and the `web/` folder are looked up
/// relative to wherever the shell happened to be — so double-clicking from
/// Explorer (CWD = C:\Windows\System32 or the user's home) would silently fail
/// to find the config and the UI. Anchoring to the exe's own directory makes
/// the program location-independent.
fn anchor_cwd_to_exe() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let _ = std::env::set_current_dir(dir);
        }
    }
}

async fn run() -> std::io::Result<()> {
    anchor_cwd_to_exe();

    // 1) Load config (config.toml -> stocktake.toml -> defaults).
    // 2) Reject empty connection strings with a clear message.
    // 3) Build the DB pool (lazy; no dial yet).
    // 4) Bind and start the HTTP server.
    let cfg = config::load()?;

    if cfg.connection_string.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "connection_string is empty. Edit config.toml (or stocktake.toml) in \
             the same folder as this program and set [database] connection_string \
             to your SQL Server details, then run it again.",
        ));
    }

    println!("🚀 Stocktake App starting...");
    println!("   📡 Listening on: http://{}:{}", cfg.host, cfg.port);
    println!(
        "   🗄️  Database: {}",
        if cfg.connection_string.contains("encrypt") {
            "configured"
        } else {
            "unencrypted"
        }
    );
    println!("   💾 Save directory: {}", cfg.output_dir);

    // Build the connection pool. This does NOT dial SQL Server — deadpool
    // connects lazily on the first request, so the server still starts (and
    // serves the UI) when the database is temporarily unreachable. A bad
    // connection only surfaces as an HTTP 500 on /api/* routes that need data.
    let pool = db::DbPool::new(&cfg.connection_string).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to build database pool: {}", e),
        )
    })?;

    let pool_data = web::Data::new(pool);
    let output_dir = web::Data::new(server::OutputDir(cfg.output_dir.clone()));
    let bind_addr = format!("{}:{}", cfg.host, cfg.port);

    println!("✅ Server ready. Open the address above in a browser.");
    println!("   (Database connects on first use — the UI loads even if SQL Server is down.)");

    // Start web server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(pool_data.clone())
            .app_data(output_dir.clone())
            // API routes are registered first so they take precedence over
            // the static-file catch-all below.
            .configure(server::configure)
            // Serve the web UI from web/ with index.html as the default file.
            .service(Files::new("/", "web").index_file("index.html"))
    })
    .bind(&bind_addr)
    .map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!(
                "Could not bind to {}: {}. Is another program already using that port, \
                 or is the host/port in config.toml wrong?",
                bind_addr, e
            ),
        )
    })?
    .run()
    .await
}
