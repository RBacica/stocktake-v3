# Stocktake v2

A Rust web application for inventory stock takes. Connects to MS SQL Server (Infinity Back Office) and provides a browser-based UI for counting inventory items.

## What's New in v2

- **Treat no count as zero** — toggle (on by default) that saves uncounted items as count 0
- **Fixed counted tracker** — items with variance are now correctly counted
- **Renamed "Recount" → "Clear"** — clears a single item's count to recount
- **Per-row Refresh** — refreshes StockOnHand for a single item from the DB
- **Refresh All** — refreshes StockOnHand for all loaded rows at once
- **Per-row Add Ticket** — click the "Ticket" button on any row to mark it for label printing; highlights purple, shows a qty input (default 1, range 1–999); click again to untoggle
- **Ticket .qry generation** — on save, ticketed items generate a `tickets/tickets-YYYY-MM-DD-HH-MM-SS.qry` file in LabelQuery format; merges with any existing `.qry` in the tickets/ folder
- **Smart .txt skip** — if only ticketed items with count 0 are saved (no genuine counts), the `.txt` count file is skipped and only the `.qry` is generated
- **Load count file** — load a previously saved stock take file back into the UI
- **Simplified table** — Department and Supplier columns removed from search results
- **SOH displayed to 2 decimal places** — e.g. `77.20` instead of `77`
- **Description lookup/filter bar** — filter displayed rows by description text
- **Parent variance fix** — parent rows show `0` variance before any count is entered (matching standalone rows)

---

## Project Structure

```
stocktake-v2/
├── Cargo.toml              # Rust package manifest (edition 2021)
├── Cargo.lock              # Locked dependency versions
├── build.rs                # Build script — embeds Windows .ico icon via windres/winres
├── config.toml             # Active config (gitignored in practice)
├── config.toml.example     # Example config template
├── stocktake.toml          # Legacy config (still supported)
├── app.rc                  # Windows resource script (icon reference)
├── app-icon.ico            # Windows application icon
├── README.md               # This file
├── src/
│   ├── main.rs             # Entry point — CLI error handling, pool init, HTTP server
│   ├── config.rs           # Config loading (config.toml → stocktake.toml → defaults)
│   ├── db.rs               # Database layer — SQL queries, connection pool, save logic
│   └── server.rs           # actix-web route handlers
├── dist/
│   ├── config.toml         # Default config shipped in the Windows zip
│   └── READ-ME-FIRST.txt   # End-user quick-start guide
└── web/
    ├── index.html          # Single-page frontend (Vanilla HTML/CSS/JS)
    ├── favicon.ico          # Browser tab icon
    ├── icon.png             # PNG icon
    └── icon.svg             # SVG icon
```

---

## Prerequisites

### All platforms

1. **Rust toolchain** (install via [rustup](https://rustup.rs/)):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
   Verify: `rustc --version` and `cargo --version`

2. **Network access to MS SQL Server** — the app connects via the TDS protocol (port 1433 by default).

### Cross-compiling for Windows (from Linux)

Install the MinGW linker and Rust Windows target:

```bash
# Arch Linux / CachyOS
sudo pacman -S mingw-w64

# Debian / Ubuntu
sudo apt install gcc-mingw-w64-x86-64
```

Add the Windows target to Rust:

```bash
rustup target add x86_64-pc-windows-gnu
```

Verify with `rustup target list --installed` — `x86_64-pc-windows-gnu` should appear.

### Native Windows build

No extra prerequisites beyond the Rust toolchain. Visual Studio Build Tools are **not** required (the project uses `native-tls` via SChannel, not OpenSSL).

---

## Step-by-Step Build Instructions

### Step 1: Get the source

```bash
# If cloning from a repository:
git clone <repo-url> stocktake-v2
cd stocktake-v2

# If working from a zip, extract it:
unzip stocktake-v2-source.zip -d stocktake-v2
cd stocktake-v2
```

### Step 2: Create `config.toml`

Copy the example and edit with your SQL Server details:

```bash
cp config.toml.example config.toml
```

Open `config.toml` in any text editor and set the connection string:

```toml
[database]
connection_string = "server=tcp:YOUR_SERVER,1433;database=YOUR_DB;uid=YOUR_USER;pwd=YOUR_PASS;encrypt=false"

[server]
host = "0.0.0.0"
port = 8080

[output]
dir = "./output"
```

> **Backslash note:** If your server uses a named instance (e.g. `MYPC\SQLEXPRESS`) or your password contains backslashes, either double them (`\\`) or wrap the value in single quotes (`'...'`).

### Step 3: Build

#### Linux native build

```bash
cargo build --release
```

Binary: `target/release/stocktake-v2`

#### Windows cross-compile (from Linux)

```bash
cargo build --release --target x86_64-pc-windows-gnu
```

Binary: `target/x86_64-pc-windows-gnu/release/stocktake-v2.exe`

#### Native Windows build

```cmd
cargo build --release
```

Binary: `target\release\stocktake-v2.exe`

### Step 4: Package for distribution (Windows)

The distribution zip requires three things in one folder:

```
stocktake.exe          ← the built binary
config.toml            ← from dist/
READ-ME-FIRST.txt      ← from dist/
web/                   ← the entire web/ folder (index.html, icons)
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

### Step 5: Run

Place the three items (exe, config.toml, web/ folder) in the same directory on the target machine, then:

- **Double-click** `stocktake.exe`, or
- Run from a terminal: `./stocktake.exe`

The console will show the URL (e.g. `http://127.0.0.1:8080`). Open it in a browser.

To stop: close the console window, or press `Ctrl+C`.

---

## Configuration Reference

### `config.toml` (new format)

```toml
[database]
# Connection string — custom key=value format or ADO.NET format
# Custom:  server=HOST,port;database=DB;uid=USER;pwd=PASS;encrypt=false
# ADO.NET: Driver={...};Server=HOST;Database=DB;Uid=USER;Pwd=PASS;
connection_string = "server=tcp:192.168.1.100,1433;database=Infinity;uid=sa;pwd=secret;encrypt=false"

[server]
host = "0.0.0.0"    # Listen on all interfaces
port = 8080          # HTTP port

[output]
dir = "./output"     # Where saved stock take files go
```

### `stocktake.toml` (legacy format, still supported)

```toml
listen_addr = "127.0.0.1"
port = 8080
connection_string = "server=tcp:192.168.1.100,1433;database=Infinity;uid=sa;pwd=secret;encrypt=false"
```

---

## Saved Output Format

Each counted row is written to a timestamped file `stocktake-YYYY-MM-DD-HH-MM-SS.txt`:

```
0,<UPC>,<Count to 4dp>,
```

Example:

```
0,1234567890123,12.0000,
0,9876543210987,0.0000,
```

### Ticket .qry file

When items have tickets, an additional `tickets/tickets-YYYY-MM-DD-HH-MM-SS.qry` file is written to the `tickets/` subdirectory. Format details:

- Each ticketed item gets a `[CriteriaN]` block with its UPC and SQL query
- Ticket qty is set via `CopiesDesc`/`SQLCopies` (default 1)
- When qty ≠ 1, `CopiesException0Code`/`CopiesException0Value` overrides are added
- If an existing `.qry` file exists in `tickets/`, it is read, its criteria extracted, and merged into the new file (re-indexed); the old file is then deleted

### Smart .txt skip

If **only** ticketed items with count 0 exist in the save (i.e. no genuine counts were entered and "treat no count as zero" is off), the `.txt` count file is **not written**. Only the `.qry` file is generated. This avoids creating empty/misleading count files when the user only wants to generate tickets.

---

## API Reference

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/departments` | List departments (DB or fallback) |
| GET | `/api/suppliers` | List suppliers (DB or fallback) |
| GET | `/api/suppliers-for-dept?department=X` | Suppliers for a department |
| GET | `/api/search?department=X&supplier=Y` | Search active items |
| GET | `/api/refresh-upc?upc=X` | Refresh StockOnHand for one item (returns `f64`) |
| POST | `/api/save` | Save counted rows as timestamped file |

### POST `/api/save` — Save counted rows

Request body (`Content-Type: application/json`):
```json
{
  "rows": [
    {
      "upc": "1234567890123",
      "description": "Product Name",
      "department": "1",
      "supplier": "001",
      "stock_on_hand": 77.20,
      "count": 12.0,
      "variance": -5.2,
      "has_ticket": true,
      "ticket_qty": 2
    }
  ]
}
```

Response:
```json
{
  "status": "ok",
  "message": "Saved 1 rows",
  "rows": 1,
  "file": "stocktake-2026-06-01-12-00-00.txt",
  "txt_written": true
}
```

- `file` is `null` when `txt_written` is `false` (only ticket-zero rows saved — no .txt generated)
- `has_ticket` and `ticket_qty` control .qry generation per row

### Item JSON format (from `/api/search`)

```json
{
  "upc": "1234567890123",
  "description": "Product Name",
  "department": "1",
  "supplier": "001",
  "stock_on_hand": 77.20,
  "parent_upc": "",
  "selling_qty": 1.0
}
```

---

## Tech Stack

- **Backend:** Rust, actix-web 4, tiberius 0.12 (TDS/SQL Server), deadpool-tiberius
- **Frontend:** Vanilla HTML/CSS/JS (served as static files from `web/`)
- **TLS:** native-tls (SChannel on Windows, no OpenSSL dependency)
- **SQL:** MS SQL Server via TDS 7.3

---

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Console flashes and closes | Startup error | Run from a terminal to see the error message |
| "config.toml exists but could not be parsed" | Unescaped backslash in config | Double backslashes or use single quotes |
| "connection_string is empty" | Config not filled in | Edit `config.toml` with your SQL Server details |
| "Could not bind to..." | Port already in use | Change `port` in `config.toml` |
| "Database error" in UI | SQL Server unreachable | Check connection string and network |
| SOH shows as integer (77 instead of 77.2) | Old binary — pre-f64 fix | Rebuild from latest source |
