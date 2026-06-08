# 🦀 Rust Programming Language — Step-by-Step Learning Guide

> **Companion project:** Stocktake v2 — a real-world Rust web application for inventory stock takes. Every concept in this guide is illustrated with code from the project.

---

## Table of Contents

1. [What is Rust & Why Learn It](#1-what-is-rust--why-learn-it)
2. [Setting Up Your Environment](#2-setting-up-your-environment)
3. [Rust Fundamentals — Concepts You Need](#3-rust-fundamentals--concepts-you-need)
4. [Understanding the Stocktake v2 Project](#4-understanding-the-stocktake-v2-project)
5. [Building the Project Step by Step](#5-building-the-project-step-by-step)
6. [Key Rust Patterns in the Project](#6-key-rust-patterns-in-the-project)
7. [Going Further — Learning Path](#7-going-further--learning-path)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. What is Rust & Why Learn It

Rust is a systems programming language that gives you **C-level performance** with **memory safety guarantees** at compile time. No garbage collector. No segfaults. No data races.

**Why it matters for this project:**
- Stocktake v2 connects to a SQL Server, serves a web UI, and writes files — all with a single ~5 MB binary
- Zero runtime dependencies — no .NET Framework, no JVM, no Node.js
- Cross-compile from Linux to Windows with a single command

**What you'll learn by studying this project:**
- How a real Rust project is structured (modules, dependencies, error handling)
- Async web servers with actix-web
- Database access with connection pooling
- File I/O and serialization
- Cross-compilation for Windows

---

## 2. Setting Up Your Environment

### 2.1 Install Rust (all platforms)

Rust is installed and managed by **rustup**, the official toolchain installer.

```bash
# Linux / macOS / Windows (PowerShell)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

After installation, restart your terminal and verify:

```bash
rustc --version    # e.g. rustc 1.78.0
cargo --version    # e.g. cargo 1.78.0
```

`rustc` is the compiler. `cargo` is the build tool and package manager — you'll use it for everything.

### 2.2 Install a Code Editor

Any text editor works, but these have excellent Rust support:

| Editor | Rust Extension |
|--------|---------------|
| VS Code | `rust-analyzer` (official) |
| Neovim | `rust-analyzer` LSP via `nvim-lspconfig` |
| IntelliJ IDEA | `Rust` plugin |

### 2.3 (Optional) Cross-compile for Windows from Linux

If you're on Linux but need to build a Windows `.exe`:

```bash
# Arch Linux / CachyOS
sudo pacman -S mingw-w64

# Debian / Ubuntu
sudo apt install gcc-mingw-w64-x86-64

# Add the Windows target to Rust
rustup target add x86_64-pc-windows-gnu

# Verify
rustup target list --installed
# Should show: x86_64-pc-windows-gnu
```

### 2.4 (Optional) Native Windows Build

Just install Rust via rustup (step 2.1). No Visual Studio Build Tools needed — this project uses `native-tls` (SChannel) instead of OpenSSL.

---

## 3. Rust Fundamentals — Concepts You Need

This section covers every Rust concept that appears in the Stocktake v2 project. Read it as a reference while exploring the code.

### 3.1 Variables and Mutability

```rust
let x = 5;        // immutable by default
let mut y = 5;    // mutable — can be reassigned
y = 6;            // OK
// x = 6;         // COMPILE ERROR — x is not mutable
```

**In the project** (`src/main.rs:48`):
```rust
let cfg = config::load()?;       // immutable — config doesn't change
let mut _buf = String::new();    // mutable — we read into it
```

### 3.2 Types

Rust is statically typed but has type inference. Key types in this project:

| Type | Example | Used for |
|------|---------|----------|
| `String` | `String::new()` | Owned, growable text |
| `&str` | `"hello"` | String slice (borrowed) |
| `i32`, `i64` | `42i64` | Integers |
| `f64` | `77.20` | Floating-point (SOH, counts) |
| `bool` | `true` | Booleans |
| `Vec<T>` | `Vec::new()` | Growable arrays |
| `Option<T>` | `Some(x)` or `None` | Values that might be absent |
| `Result<T, E>` | `Ok(x)` or `Err(e)` | Operations that might fail |

### 3.3 Functions

```rust
fn add(a: i32, b: i32) -> i32 {
    a + b  // no semicolon = return value
}
```

The `-> i32` is the return type. The last expression (no semicolon) is what gets returned.

**In the project** (`src/main.rs:26-31`):
```rust
fn pause_before_exit() {
    print!("Press Enter to close this window... ");
    let _ = std::io::stdout().flush();
    let mut _buf = String::new();
    let _ = std::io::stdin().read_line(&mut _buf);
}
```

### 3.4 The Ownership System (Rust's Superpower)

Every value has exactly one **owner**. When the owner goes out of scope, the value is freed.

```rust
let s1 = String::from("hello");
let s2 = s1;       // s1 is MOVED to s2
// println!("{}", s1);  // COMPILE ERROR — s1 no longer valid
println!("{}", s2);    // OK
```

**Borrowing** lets you use a value without taking ownership:

```rust
fn length(s: &String) -> usize {  // & = borrow (reference)
    s.len()
}  // s goes out of scope but the original String is NOT freed
```

**In the project** (`src/config.rs`):
```rust
pub struct Config {
    pub connection_string: String,  // Config OWNS this String
}

fn load() -> Result<Config> { ... }  // ownership moves to caller
```

### 3.5 Structs

Structs group related data together — like a class without methods.

**In the project** (`src/config.rs`):
```rust
pub struct Config {
    pub connection_string: String,
    pub host: String,
    pub port: u16,
    pub output_dir: String,
}
```

The `pub` keyword makes fields accessible from other modules.

### 3.6 Enums and Pattern Matching

Enums let a value be one of several variants. `match` handles each case.

```rust
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle(r) => 3.14159 * r * r,
        Shape::Rectangle(w, h) => w * h,
    }
}
```

### 3.7 Option and Result — Error Handling

Rust doesn't have exceptions. Instead:

- `Option<T>` = value might be `Some(x)` or `None`
- `Result<T, E>` = operation might succeed `Ok(x)` or fail `Err(e)`

The `?` operator propagates errors automatically:

```rust
fn load() -> Result<Config> {
    let content = std::fs::read_to_string("config.toml")?;  // If Err, return it
    let cfg: Config = toml::from_str(&content)?;             // Same here
    Ok(cfg)                                                   // Success
}
```

**In the project** (`src/main.rs:14`):
```rust
if let Err(e) = run().await {
    eprintln!("❌ Stocktake failed to start:");
    eprintln!("   {}", e);
    pause_before_exit();
    std::process::exit(1);
}
```

`if let Err(e)` runs the block only if `run()` returned an error.

### 3.8 Traits

Traits define shared behavior — like interfaces in other languages.

```rust
trait Printable {
    fn print(&self);
}

impl Printable for Config {
    fn print(&self) {
        println!("Server: {}:{}", self.host, self.port);
    }
}
```

Key traits used in this project:
- `Serialize` / `Deserialize` (from `serde`) — convert structs to/from JSON
- `Clone` — make a copy of a value
- `Default` — provide default values

### 3.9 Modules

Rust code is organized into modules. The project has four:

```
src/
├── main.rs      # Entry point (mod config; mod db; mod server;)
├── config.rs    # Configuration loading
├── db.rs        # Database layer
└── server.rs    # HTTP route handlers
```

In `src/main.rs`:
```rust
mod config;   // Include config.rs
mod db;       // Include db.rs
mod server;   // Include server.rs
```

Items are accessed with `::` — e.g., `config::load()`, `db::DbPool::new()`.

### 3.10 Async / Await

Async lets the program handle many connections at once without blocking.

```rust
async fn run() -> std::io::Result<()> {
    let pool = db::DbPool::new(&cfg.connection_string).map_err(...)?;
    // ... start server
}
```

The `#[actix_web::main]` attribute on `main()` tells actix to run the async runtime.

### 3.11 Closures

Closures are anonymous functions that can capture variables from their scope.

**In the project** (`src/main.rs:96-106`):
```rust
HttpServer::new(move || {
    // This is a closure — `move` transfers ownership of captured variables
    App::new()
        .wrap(Logger::default())
        .app_data(pool_data.clone())
        .configure(server::configure)
        .service(Files::new("/", "web").index_file("index.html"))
})
```

### 3.12 Iterators and Collections

```rust
let numbers = vec![1, 2, 3, 4, 5];
let doubled: Vec<i32> = numbers.iter().map(|x| x * 2).collect();
// doubled = [2, 4, 6, 8, 10]
```

**In the project** (`src/db.rs` — save logic):
```rust
for row in &rows {
    writeln!(file, "0,{},{:.4},", row.upc, row.count)?;
}
```

---

## 4. Understanding the Stocktake v2 Project

### 4.1 What the Application Does

1. **Starts a web server** (actix-web) on `http://0.0.0.0:8080`
2. **Serves a single-page UI** (`web/index.html`) — vanilla HTML/CSS/JS
3. **Connects to MS SQL Server** (via tiberius/TDS) to fetch inventory items
4. **Lets users count items** in the browser, then save results to timestamped `.txt` files
5. **Generates `.qry` ticket files** for label printing when items are ticketed

### 4.2 Architecture Diagram

```
┌─────────────────────────────────────────────────┐
│                  Browser (UI)                    │
│           web/index.html (Vanilla JS)            │
│  ┌──────────┐ ┌──────────┐ ┌─────────────────┐  │
│  │ Search   │ │ Count    │ │ Save / Tickets  │  │
│  │ Dept/Sup │ │ Variance │ │ .txt + .qry     │  │
│  └────┬─────┘ └────┬─────┘ └───────┬─────────┘  │
└───────┼──────────────┼──────────────┼────────────┘
        │  fetch()     │              │  POST /api/save
        ▼              ▼              ▼
┌─────────────────────────────────────────────────┐
│            actix-web HTTP Server                 │
│  ┌──────────────────────────────────────────┐   │
│  │  server.rs — Route Handlers               │   │
│  │  GET  /api/departments                    │   │
│  │  GET  /api/suppliers                      │   │
│  │  GET  /api/search?dept=X&sup=Y            │   │
│  │  GET  /api/refresh-upc?upc=X              │   │
│  │  POST /api/save                           │   │
│  └──────────────┬───────────────────────────┘   │
│                 │                                │
│  ┌──────────────▼───────────────────────────┐   │
│  │  db.rs — Database Layer                   │   │
│  │  • Connection pool (deadpool-tiberius)    │   │
│  │  • SQL queries (tiberius/TDS)             │   │
│  │  • Save logic (.txt + .qry generation)    │   │
│  └──────────────┬───────────────────────────┘   │
│                 │                                │
│  ┌──────────────▼───────────────────────────┐   │
│  │  config.rs — Configuration                │   │
│  │  • Reads config.toml / stocktake.toml     │   │
│  │  • Falls back to defaults                 │   │
│  └──────────────────────────────────────────┘   │
└──────────────────────┬──────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │  MS SQL Server │
              │  (Infinity DB) │
              └────────────────┘
```

### 4.3 File-by-File Walkthrough

#### `Cargo.toml` — The Project Manifest

```toml
[package]
name = "stocktake-v2"
version = "2.0.0"
edition = "2021"

[dependencies]
actix-web = "4"           # Web framework
actix-files = "0.6"       # Static file serving
tokio = { version = "1", features = ["full"] }  # Async runtime
tiberius = { version = "0.12", features = ["tds73", "native-tls", "chrono"] }  # SQL Server driver
serde = { version = "1", features = ["derive"] }  # JSON serialization
serde_json = "1"          # JSON parsing
toml = "0.8"              # TOML config parsing
thiserror = "2"           # Error type macros
deadpool-tiberius = "0.1" # Connection pooling
futures-util = "0.3"      # Async utilities
chrono = "0.4"            # Date/time formatting

[target.'cfg(windows)'.build-dependencies]
winres = "0.1"            # Embed Windows icon in .exe
```

**Key concept:** Dependencies are fetched from [crates.io](https://crates.io) (Rust's package registry) and compiled from source. `Cargo.lock` pins exact versions.

#### `src/main.rs` — Entry Point (120 lines)

The `main()` function:
1. Calls `run()` — if it errors, prints the error and pauses (so the console window doesn't flash closed on Windows)
2. `run()` does: load config → create DB pool → start HTTP server

Key patterns:
- `#[actix_web::main]` — attribute macro that sets up the async runtime
- `if let Err(e)` — pattern match on error only
- `?` operator — early-return on error
- `move ||` — closure that captures ownership for the web server factory

#### `src/config.rs` — Configuration Loading

Defines the `Config` struct and `load()` function:
- Tries `config.toml` first, then `stocktake.toml` (legacy), then defaults
- Uses `toml` crate to parse TOML into the `Config` struct
- Returns `Result<Config>` — callers use `?` to propagate errors

#### `src/db.rs` — Database Layer

The largest module. Contains:
- `DbPool` — connection pool wrapper around `deadpool-tiberius`
- `get_departments()`, `get_suppliers()`, `search_items()` — SQL queries
- `save_stocktake()` — writes `.txt` count files and `.qry` ticket files
- `refresh_upc()` — re-queries SOH for a single item

Key patterns:
- `async fn` — all DB operations are async
- `&self` — methods borrow the pool (don't take ownership)
- `?` — every DB call can fail; errors propagate up
- SQL parameter binding — `query.bind(&param)` prevents SQL injection

#### `src/server.rs` — HTTP Route Handlers

Defines all API endpoints using actix-web's `web::resource()` routing:
- Each handler is an `async fn` that returns `impl Responder`
- Extracts query params with `web::Query<T>` and path data with `web::Json<T>`
- Calls into `db.rs` functions and returns JSON responses

#### `web/index.html` — Frontend (single file, ~1500 lines)

Vanilla HTML/CSS/JS — no frameworks. Communicates with the backend via `fetch()`:
- `GET /api/search?dept=X&sup=Y` → populates the table
- `POST /api/save` with JSON body → saves counts
- `GET /api/refresh-upc?upc=X` → refreshes a single row's SOH

---

## 5. Building the Project Step by Step

### Step 1: Get the Source

```bash
# From a zip:
unzip stocktake-v2-source.zip -d stocktake-v2
cd stocktake-v2

# Or from git:
git clone <repo-url> stocktake-v2
cd stocktake-v2
```

### Step 2: Understand the Project Structure

```bash
# List all files
find . -type f | sort

# Key files you'll see:
# Cargo.toml          — dependencies and metadata
# src/main.rs         — entry point
# src/config.rs       — config loading
# src/db.rs           — database + save logic
# src/server.rs       — HTTP routes
# web/index.html      — frontend
# config.toml.example — template for your config
```

### Step 3: Create Your Config

```bash
cp config.toml.example config.toml
```

Edit `config.toml` with your SQL Server details:

```toml
[database]
connection_string = "server=tcp:YOUR_SERVER,1433;database=YOUR_DB;uid=YOUR_USER;pwd=YOUR_PASS;encrypt=false"

[server]
host = "0.0.0.0"
port = 8080

[output]
dir = "./output"
```

> **Note:** If your password has backslashes, either double them (`\\`) or wrap the value in single quotes (`'p@$$w0rd\\123'`).

### Step 4: Build (First Time)

```bash
# Linux native
cargo build --release

# Windows cross-compile (from Linux)
cargo build --release --target x86_64-pc-windows-gnu

# Native Windows
cargo build --release
```

**What happens during the first build:**
1. Cargo reads `Cargo.toml` and downloads all dependencies from crates.io
2. Compiles each dependency from source (this takes a few minutes)
3. Compiles your `src/` code
4. Links everything into a single binary

Subsequent builds are much faster — Cargo only recompiles what changed.

**Where the binary ends up:**
| Build type | Binary path |
|-----------|-------------|
| Linux native | `target/release/stocktake-v2` |
| Windows cross | `target/x86_64-pc-windows-gnu/release/stocktake-v2.exe` |
| Windows native | `target\release\stocktake-v2.exe` |

### Step 5: Run

```bash
# Linux / macOS
./target/release/stocktake-v2

# Windows (cross-compiled, run on Windows)
stocktake-v2.exe
```

You should see:
```
🚀 Stocktake App starting...
   📡 Listening on: http://0.0.0.0:8080
   🗄️  Database: configured
   💾 Save directory: ./output
✅ Server ready. Open the address above in a browser.
```

Open `http://127.0.0.1:8080` in your browser.

### Step 6: Package for Distribution (Windows)

The distribution needs these files together:

```
stocktake.exe              ← the built binary
config.toml                ← from dist/
READ-ME-FIRST.txt          ← from dist/
web/
├── index.html
├── favicon.ico
├── icon.png
└── icon.svg
```

Create the zip:

```bash
mkdir -p dist-package/web
cp target/x86_64-pc-windows-gnu/release/stocktake-v2.exe dist-package/stocktake.exe
cp dist/config.toml dist-package/
cp dist/READ-ME-FIRST.txt dist-package/
cp web/index.html dist-package/web/
cp web/favicon.ico dist-package/web/
cp web/icon.png dist-package/web/
cp web/icon.svg dist-package/web/
cd dist-package && zip -r ../stocktake-v2-windows.zip .
```

### Step 7: Make a Change and Rebuild

Try a simple change — edit `src/main.rs` line 65:

```rust
// Change this:
println!("🚀 Stocktake App starting...");
// To this:
println!("🚀 Stocktake v2 — starting up...");
```

Then rebuild:

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

Cargo will only recompile `main.rs` and relink — takes seconds, not minutes.

---

## 6. Key Rust Patterns in the Project

### Pattern 1: Error Propagation with `?`

```rust
async fn run() -> std::io::Result<()> {
    let cfg = config::load()?;           // If load fails, return the error
    let pool = db::DbPool::new(&cfg.connection_string).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::Other, format!("...", e))
    })?;
    // ... if we get here, everything succeeded
}
```

The `?` operator is Rust's idiomatic error handling. Instead of try/catch, you mark functions as returning `Result` and use `?` to propagate failures.

### Pattern 2: Structs with `serde` for Serialization

```rust
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub connection_string: String,
    pub host: String,
    pub port: u16,
}
```

`#[derive(Serialize, Deserialize)]` auto-generates code to convert the struct to/from JSON, TOML, etc. This is how `config.toml` gets loaded into a `Config` struct.

### Pattern 3: Connection Pooling

```rust
// In db.rs:
pub struct DbPool {
    pool: Pool<TiberiusConnectionManager>,
}

impl DbPool {
    pub fn new(conn_str: &str) -> Result<Self> {
        // Create pool — connections are established lazily
    }
}
```

The pool manages a set of reusable database connections. When a request needs DB access, it checks out a connection from the pool, uses it, and returns it.

### Pattern 4: Async Route Handlers

```rust
// In server.rs:
async fn search_items(
    pool: web::Data<DbPool>,
    query: web::Query<SearchQuery>,
) -> impl Responder {
    match db::search_items(&pool, &query.department, &query.supplier).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => HttpResponse::InternalServerError().body(format!("Database error: {}", e)),
    }
}
```

Each handler is async, extracts data from the request (query params, JSON body, shared state), calls the DB layer, and returns an HTTP response.

### Pattern 5: File I/O with Error Handling

```rust
// In db.rs — save_stocktake():
let mut file = File::create(&filepath)?;
for row in &rows {
    writeln!(file, "0,{},{:.4},", row.upc, row.count)?;
}
```

`File::create()` returns `Result<File>`. The `?` propagates errors. `writeln!` is a macro like `println!` but writes to a file.

### Pattern 6: Cross-Platform Conditional Compilation

```rust
// In Cargo.toml:
[target.'cfg(windows)'.build-dependencies]
winres = "0.1"

// In build.rs:
#[cfg(windows)]
fn main() {
    // Only runs on Windows builds — embeds the .ico icon
}
```

`#[cfg(windows)]` means "only compile this code when targeting Windows."

---

## 7. Going Further — Learning Path

### Beginner (Week 1-2)
1. **[The Rust Book](https://doc.rust-lang.org/book/)** — the official, free tutorial. Read chapters 1-6.
2. **[Rust by Example](https://doc.rust-lang.org/rust-by-example/)** — learn by reading and modifying code.
3. **Exercise:** Modify the Stocktake v2 welcome message in `main.rs`, rebuild, and run.

### Intermediate (Week 3-4)
1. **The Rust Book** — chapters 7-13 (error handling, modules, collections, traits).
2. **[Rustlings](https://github.com/rust-lang/rustlings)** — small exercises that teach Rust syntax.
3. **Exercise:** Add a new API endpoint to `server.rs` (e.g., `/api/health` that returns `{"status": "ok"}`).

### Advanced (Week 5+)
1. **The Rust Book** — chapters 14-20 (closures, iterators, smart pointers, concurrency, macros).
2. **[Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)** — deep dive into async/await.
3. **Exercise:** Add a new feature to Stocktake v2 — e.g., export counts as CSV.

### Recommended Resources

| Resource | Type | Cost |
|----------|------|------|
| [The Rust Book](https://doc.rust-lang.org/book/) | Official tutorial | Free |
| [Rust by Example](https://doc.rust-lang.org/rust-by-example/) | Interactive examples | Free |
| [Rustlings](https://github.com/rust-lang/rustlings) | Exercises | Free |
| [Rust Cookbook](https://rust-lang-nursery.github.io/rust-cookbook/) | Code recipes | Free |
| [crates.io](https://crates.io) | Package registry | Free |
| [docs.rs](https://docs.rs) | Auto-generated docs for all crates | Free |

---

## 8. Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| `cargo: command not found` | Rust not installed or PATH not set | Run `source ~/.cargo/env` or restart terminal |
| `linking with 'cc' failed` | No C compiler installed | Install `gcc` (Linux) or use `x86_64-pc-windows-gnu` target |
| `failed to download` | Network/proxy issue | Check internet; try `cargo fetch` first |
| `unresolved import` | Missing dependency | Add the crate to `[dependencies]` in `Cargo.toml` |
| `cannot move out of borrowed content` | Ownership error | Read §3.4 — you likely need to `.clone()` or use `&` |
| `expected X, found Y` | Type mismatch | Check the types; Rust is strict about conversions |
| Console flashes and closes | Startup error | Run from a terminal to see the error message |
| `config.toml exists but could not be parsed` | Unescaped backslash | Double backslashes or use single quotes |
| `connection_string is empty` | Config not filled in | Edit `config.toml` with your SQL Server details |
| `Could not bind to...` | Port already in use | Change `port` in `config.toml` |

---

## Quick Reference Card

```bash
# Create a new project
cargo new my-project
cd my-project

# Build (debug — fast compile, slow runtime)
cargo build

# Build (release — slow compile, fast runtime)
cargo build --release

# Build and run in one step
cargo run

# Run tests
cargo test

# Check code without building (fast)
cargo check

# Update dependencies
cargo update

# Format code
cargo fmt

# Lint
cargo clippy

# Cross-compile for Windows
cargo build --release --target x86_64-pc-windows-gnu
```

---

*Guide written for the Stocktake v2 project. All code examples are from the actual source. Last updated: June 2026.*
