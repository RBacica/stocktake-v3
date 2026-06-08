use deadpool_tiberius::Manager;
use futures_util::StreamExt;
use tiberius::{EncryptionLevel, QueryItem};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StockItem {
    pub upc: String,
    pub description: String,
    pub department: String,
    pub supplier: String,
    pub stock_on_hand: f64,
    pub parent_upc: String,
    pub selling_qty: f64,
}

/// Query string params for `/api/search` (department / supplier, both default ALL).
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub department: Option<String>,
    pub supplier: Option<String>,
}

/// Query string params for `/api/refresh-upc` (single UPC lookup).
#[derive(Debug, Deserialize)]
pub struct UpcQuery {
    pub upc: Option<String>,
}

/// Query string params for `/api/suppliers-for-dept` (department filter).
#[derive(Debug, Deserialize)]
pub struct DeptQuery {
    pub department: Option<String>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SaveRow {
    pub upc: String,
    pub description: String,
    #[serde(default)]
    pub department: String,
    #[serde(default)]
    pub supplier: String,
    pub stock_on_hand: f64,
    pub count: f64,
    pub variance: f64,
    #[serde(default)]
    pub has_ticket: bool,
    #[serde(default = "default_ticket_qty")]
    pub ticket_qty: i32,
}

fn default_ticket_qty() -> i32 { 1 }

/// Body of `/api/save`: the full set of counted rows for a stock take.
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveRequest {
    pub rows: Vec<SaveRow>,
}

/// A department option. `department` is the *value* (the ID stored in Infinity's
/// Departments table, e.g. "1"); `label` is the human-friendly text shown in the
/// dropdown (e.g. "1 Beer"). Mirrors `Supplier` exactly. When value and label are
/// identical (an unrecognised DB-sourced row) both fields carry the same string.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Department {
    pub department: String,
    pub label: String,
}

/// A supplier option. `supplier` is the *value* (the code stored in the SQL
/// `Supplier` column, e.g. "001"); `label` is the human-friendly text shown in
/// the dropdown (e.g. "001 Tasman"). When the value and label are identical
/// (as they are for DB-sourced rows) both fields just carry the same string.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Supplier {
    pub supplier: String,
    pub label: String,
}

/// The fixed Infinity Back Office departments. Used as a fallback so the
/// dropdown is always populated with production-correct values, even before
/// SQL Server is reachable. When the live DB returns its own distinct list,
/// that takes precedence.
/// The fixed Infinity Back Office departments as (id, name) pairs. The id is the
/// value stored in Infinity's `Department` column and written to the saved CSV;
/// the display label is "id name". Used as a fallback so the dropdown is always
/// populated with production-correct values, even before SQL Server is reachable.
/// When the live DB returns its own distinct list, that takes precedence.
pub const FALLBACK_DEPARTMENTS: &[(&str, &str)] = &[
    ("1", "Beer"),
    ("55", "Cider"),
    ("56", "Cigarettes/Tobacco"),
    ("57", "Liqueur"),
    ("58", "Miscellaneous"),
    ("59", "Non Alcoholic Drinks"),
    ("61", "Ready To Drink"),
    ("62", "Snacks/Deli"),
    ("63", "Spirits"),
    ("64", "Wines"),
];

/// The known suppliers as (code, name) pairs. The code is the value matched
/// against the SQL `Supplier` column and written to the saved CSV; the display
/// label is "code name". Fallback list — the live DB wins when available.
pub const FALLBACK_SUPPLIERS: &[(&str, &str)] = &[
    ("001", "Tasman"),
    ("31", "Asahi"),
    ("32", "Lion"),
    ("33", "Hancocks"),
];

fn fallback_departments() -> Vec<Department> {
    FALLBACK_DEPARTMENTS
        .iter()
        .map(|(id, name)| Department {
            department: id.to_string(),
            label: format!("{} {}", id, name),
        })
        .collect()
}

fn fallback_suppliers() -> Vec<Supplier> {
    FALLBACK_SUPPLIERS
        .iter()
        .map(|(code, name)| Supplier {
            supplier: code.to_string(),
            label: format!("{} {}", code, name),
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Connection failed: {0}")]
    Connection(String),
    #[error("Query failed: {0}")]
    Query(String),
}

/// Coerce a single column cell to a String regardless of its SQL type.
///
/// Infinity stores some "code" columns (Department, Supplier) as integer
/// types (SMALLINT/INT) rather than strings, so a blind `try_get::<&str>`
/// fails with "cannot interpret I16(..) as a String value". We try the
/// string type first, then the common integer widths, and finally fall back
/// to an empty string for NULL / unrecognised types. The result is trimmed.
fn cell_to_string(row: &tiberius::Row, idx: usize) -> String {
    if let Ok(Some(s)) = row.try_get::<&str, _>(idx) {
        return s.trim().to_string();
    }
    if let Ok(Some(v)) = row.try_get::<i32, _>(idx) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<i16, _>(idx) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<i64, _>(idx) {
        return v.to_string();
    }
    if let Ok(Some(v)) = row.try_get::<u8, _>(idx) {
        return v.to_string();
    }
    String::new()
}

/// Coerce a single column cell to an f64 regardless of SQL numeric type.
fn cell_to_f64(row: &tiberius::Row, idx: usize) -> f64 {
    if let Ok(Some(v)) = row.try_get::<f64, _>(idx) {
        return v;
    }
    if let Ok(Some(v)) = row.try_get::<f32, _>(idx) {
        return v as f64;
    }
    if let Ok(Some(v)) = row.try_get::<i64, _>(idx) {
        return v as f64;
    }
    if let Ok(Some(v)) = row.try_get::<i32, _>(idx) {
        return v as f64;
    }
    if let Ok(Some(v)) = row.try_get::<i16, _>(idx) {
        return v as f64;
    }
    if let Ok(Some(v)) = row.try_get::<u8, _>(idx) {
        return v as f64;
    }
    // SQL Server numeric/decimal may arrive as tiberius::Numeric;
    // try via string parsing as a last resort.
    if let Ok(Some(s)) = row.try_get::<&str, _>(idx) {
        if let Ok(v) = s.trim().parse::<f64>() {
            return v;
        }
    }
    0.0
}

/// Parse our custom config format:
///   server=HOST,port;database=DB;uid=USER;pwd=PASS;encrypt=false
/// Falls back to ADO.NET style if "Driver=" is detected.
pub fn build_manager(conn_string: &str) -> Result<Manager, DbError> {
    let lower = conn_string.to_lowercase();
    
    // ADO.NET style - let tiberius parse it
    if lower.contains("driver=") || lower.contains("server={") {
        return Manager::from_ado_string(conn_string)
            .map_err(|e| DbError::Connection(e.to_string()));
    }
    
    // Custom key=value style
    let mut manager = Manager::new();
    let mut username: Option<String> = None;
    let mut password: Option<String> = None;
    
    for pair in conn_string.split(';') {
        let pair = pair.trim();
        if pair.is_empty() { continue; }
        let kv: Vec<&str> = pair.splitn(2, '=').collect();
        if kv.len() != 2 { continue; }
        let key = kv[0].trim().to_lowercase();
        let val = kv[1].trim();
        match key.as_str() {
            "server" | "host" => {
                let s = val.trim_start_matches("tcp:").trim();
                if let Some(comma_pos) = s.find(',') {
                    let host = s[..comma_pos].trim().trim_matches('[').trim_matches(']').to_string();
                    let port: u16 = s[comma_pos+1..].trim().parse().unwrap_or(1433);
                    manager = manager.host(host).port(port);
                } else {
                    manager = manager.host(s.trim_matches('[').trim_matches(']'));
                }
            }
            "port" => { manager = manager.port(val.parse().unwrap_or(1433)); }
            "uid" | "user" | "username" => { username = Some(val.to_string()); }
            "pwd" | "password" => { password = Some(val.to_string()); }
            "database" | "db" => { manager = manager.database(val); }
            "encrypt" => {
                let on = val.to_lowercase() != "off" && val.to_lowercase() != "false" && val.to_lowercase() != "no";
                manager = manager.encryption(if on { EncryptionLevel::Required } else { EncryptionLevel::Off });
            }
            "trust_cert" => {
                if val.to_lowercase() == "true" || val.to_lowercase() == "yes" {
                    manager = manager.trust_cert();
                }
            }
            _ => {}
        }
    }
    
    if let (Some(u), Some(p)) = (username, password) {
        manager = manager.basic_authentication(u, p);
    }
    
    Ok(manager)
}

#[derive(Clone)]
pub struct DbPool {
    pool: deadpool_tiberius::Pool,
}

impl DbPool {
    /// Build the connection pool. This does **not** open a TCP/TLS connection
    /// to SQL Server — `deadpool` connects lazily, on the first `pool.get()`
    /// inside a request handler. That means the web server can start and serve
    /// the UI even when the database is temporarily unreachable; the DB error
    /// only surfaces (as an HTTP 500) when a route that needs data is called.
    pub fn new(conn_string: &str) -> Result<Self, DbError> {
        let manager = build_manager(conn_string)?;
        let pool = manager
            .max_size(10)
            .trust_cert()
            .create_pool()
            .map_err(|e| DbError::Connection(e.to_string()))?;
        Ok(Self { pool })
    }

    pub async fn get_departments(&self) -> Result<Vec<Department>, DbError> {
        let mut conn = self.pool.get().await.map_err(|e| DbError::Connection(e.to_string()))?;
        // ID is the value stored in Items.Department and written to the saved
        // file; Description is the human name. Display format: "Description (ID)".
        // Exclude Non Sales and EPay from the list.
        let mut stream = conn.query(
            "SELECT ID, Description FROM Departments WHERE Description NOT IN ('Non Sales', 'EPay')",
            &[],
        ).await.map_err(|e| DbError::Query(e.to_string()))?;
        
        let mut results = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                let id = cell_to_string(&row, 0);
                if id.is_empty() {
                    continue;
                }
                let description = cell_to_string(&row, 1);
                // Display "Description (ID)"; if Description is blank fall back to
                // just the raw id so the option is never empty.
                let label = if description.is_empty() {
                    id.clone()
                } else {
                    format!("{} ({})", description, id)
                };
                results.push(Department { department: id, label });
            }
        }
        Ok(results)
    }

    pub async fn get_suppliers(&self) -> Result<Vec<Supplier>, DbError> {
        let mut conn = self.pool.get().await.map_err(|e| DbError::Connection(e.to_string()))?;
        // Code is the value matched against Items.Supplier and written to the
        // saved file. Display format: "Code - LastName (FirstName)".
        let mut stream = conn.query(
            "SELECT Code, LastName, FirstName FROM Customers WHERE CustType = 'R' AND InActive = '0'",
            &[],
        ).await.map_err(|e| DbError::Query(e.to_string()))?;
        
        let mut results = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                let code = cell_to_string(&row, 0);
                if code.is_empty() {
                    continue;
                }
                let last_name = cell_to_string(&row, 1);
                let first_name = cell_to_string(&row, 2);
                // "Code - LastName (FirstName)". Degrade gracefully when names are
                // blank so the option is never empty or full of stray punctuation.
                let label = match (last_name.is_empty(), first_name.is_empty()) {
                    (false, false) => format!("{} - {} ({})", code, last_name, first_name),
                    (false, true) => format!("{} - {}", code, last_name),
                    (true, false) => format!("{} - ({})", code, first_name),
                    (true, true) => code.clone(),
                };
                results.push(Supplier { supplier: code, label });
            }
        }
        Ok(results)
    }

    /// Departments, preferring the live DB list but falling back to the fixed
    /// Infinity list if the DB is unreachable or returns nothing. Always
    /// returns a populated list so the UI dropdown is never empty.
    pub async fn departments_or_fallback(&self) -> Vec<Department> {
        match self.get_departments().await {
            Ok(list) if !list.is_empty() => list,
            Ok(_) => fallback_departments(),
            Err(e) => {
                eprintln!("get_departments failed ({e}); using fallback list");
                fallback_departments()
            }
        }
    }

    /// Suppliers, preferring the live DB list but falling back to the four
    /// known suppliers if the DB is unreachable or returns nothing.
    pub async fn suppliers_or_fallback(&self) -> Vec<Supplier> {
        match self.get_suppliers().await {
            Ok(list) if !list.is_empty() => list,
            Ok(_) => fallback_suppliers(),
            Err(e) => {
                eprintln!("get_suppliers failed ({e}); using fallback list");
                fallback_suppliers()
            }
        }
    }

    /// Fetch the latest StockOnHand for a single UPC.
    /// Returns the stock_on_hand value, or None if the UPC is not found.
    pub async fn refresh_upc(&self, upc: &str) -> Result<f64, DbError> {
        let mut conn = self.pool.get().await.map_err(|e| DbError::Connection(e.to_string()))?;
        let safe_upc = upc.replace('\'', "''");
        let query = format!(
            "SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT) AS StockOnHand \
             FROM [ItemMovement] m \
             WHERE m.UPC = '{}' \
             ORDER BY m.ID DESC",
            safe_upc
        );
        let mut stream = conn.query(&query, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                return Ok(cell_to_f64(&row, 0));
            }
        }
        Ok(0.0)
    }

    /// Fetch suppliers that appear in items belonging to a given department.
    /// If department is "ALL" or empty, returns all suppliers (same as get_suppliers).
    pub async fn get_suppliers_for_department(&self, department: &str) -> Result<Vec<Supplier>, DbError> {
        // If no department filter, return all suppliers
        if department == "ALL" || department.is_empty() {
            return self.get_suppliers().await;
        }

        let mut conn = self.pool.get().await.map_err(|e| DbError::Connection(e.to_string()))?;
        let safe_dept = department.replace('\'', "''");
        let query = format!(
            "SELECT DISTINCT i.Supplier \
             FROM Items i \
             WHERE i.InActive = '0' AND i.Department = '{}' AND i.Supplier IS NOT NULL AND i.Supplier <> '' \
             ORDER BY i.Supplier",
            safe_dept
        );
        let mut stream = conn.query(&query, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut supplier_codes: Vec<String> = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                let code = cell_to_string(&row, 0);
                if !code.is_empty() {
                    supplier_codes.push(code);
                }
            }
        }
        // Drop the first stream so we can reuse conn
        drop(stream);

        if supplier_codes.is_empty() {
            return Ok(Vec::new());
        }

        // Now fetch full supplier details (name etc) for these codes
        let codes_list = supplier_codes.iter()
            .map(|c| format!("'{}'", c.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(",");
        let query2 = format!(
            "SELECT Code, LastName, FirstName \
             FROM Customers \
             WHERE CustType = 'R' AND InActive = '0' AND Code IN ({}) \
             ORDER BY Code",
            codes_list
        );
        let mut stream2 = conn.query(&query2, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let mut results = Vec::new();
        while let Some(item) = stream2.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                let code = cell_to_string(&row, 0);
                if code.is_empty() { continue; }
                let last_name = cell_to_string(&row, 1);
                let first_name = cell_to_string(&row, 2);
                let label = match (last_name.is_empty(), first_name.is_empty()) {
                    (false, false) => format!("{} - {} ({})", code, last_name, first_name),
                    (false, true) => format!("{} - {}", code, last_name),
                    (true, false) => format!("{} - ({})", code, first_name),
                    (true, true) => code.clone(),
                };
                results.push(Supplier { supplier: code, label });
            }
        }
        Ok(results)
    }

    pub async fn search_items(&self, department: &str, supplier: &str) -> Result<Vec<StockItem>, DbError> {
        let mut conn = self.pool.get().await.map_err(|e| DbError::Connection(e.to_string()))?;
        let dept_clause = if department != "ALL" && !department.is_empty() {
            format!(" AND i.Department = '{}'", department.replace('\'', "''"))
        } else {
            String::new()
        };
        let sup_clause = if supplier != "ALL" && !supplier.is_empty() {
            format!(" AND i.Supplier = '{}'", supplier.replace('\'', "''"))
        } else {
            String::new()
        };
        
        let query = format!(
            "SELECT
                i.UPC,
                i.[Description],
                i.[Department],
                i.[Supplier],
                (
                    SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT)
                    FROM [ItemMovement] m
                    WHERE m.UPC = i.UPC
                    ORDER BY m.ID Desc
                ) AS StockOnHand,
                ISNULL(i.ParentUPC, '') AS ParentUPC,
                ISNULL(i.SellingQty, 0) AS SellingQty
            FROM Items i
            WHERE i.InActive = '0'
                {}
                {}
            ORDER BY i.[Description]",
            dept_clause, sup_clause
        );
        
        let mut stream = conn.query(&query, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        
        let mut items = Vec::new();
        while let Some(item) = stream.next().await {
            let item = item.map_err(|e| DbError::Query(e.to_string()))?;
            if let QueryItem::Row(row) = item {
                items.push(StockItem {
                    upc: cell_to_string(&row, 0),
                    description: cell_to_string(&row, 1),
                    department: cell_to_string(&row, 2),
                    supplier: cell_to_string(&row, 3),
                    stock_on_hand: cell_to_f64(&row, 4),
                    parent_upc: cell_to_string(&row, 5),
                    selling_qty: cell_to_f64(&row, 6),
                });
            }
        }
        Ok(items)
    }
}

/// Save a completed stock take to a timestamped `.txt` file.
///
/// Each counted row is written as a single line in the exact format the
/// downstream import expects:
///
///     0,<UPC>,<Count>,
///
/// where the leading `0` is a static literal, `<UPC>` is the item UPC, and
/// `<Count>` is the counted quantity formatted to 4 decimal places (e.g. a
/// count of 12 -> `12.0000`, a count of 0 -> `0.0000`). There is a trailing
/// comma at the end of every line and no header row or extra columns.
///
/// Files are written into `output_dir` (created if missing). Returns the
/// absolute path of the file written, for reporting back to the browser.
///
/// When any row has `has_ticket = true`, an additional LabelQuery `.qry` file
/// is written to `<output_dir>/tickets/`. The .qry file contains one Criteria
/// block per ticketed item, with the item's UPC and ticket_qty (via
/// CopiesException overrides when qty ≠ 1).
pub fn save_stocktake(output_dir: &str, rows: &[SaveRow]) -> Result<(std::path::PathBuf, bool), std::io::Error> {
    use std::io::Write;

    std::fs::create_dir_all(output_dir)?;

    // stocktake-YYYY-MM-DD-HH-MM-SS.txt
    let now = chrono::Local::now();
    let timestamp = now.format("%Y-%m-%d-%H-%M-%S");
    let fname = format!("stocktake-{}.txt", timestamp);
    let path = std::path::Path::new(output_dir).join(&fname);

    // Only write the .txt count file if there are genuine count rows
    // (rows with a non-zero count or rows without tickets).
    // Ticket-only rows with count 0 are for .qry generation only.
    let genuine_rows: Vec<&SaveRow> = rows.iter()
        .filter(|r| r.count != 0.0 || !r.has_ticket)
        .collect();

    let txt_written;
    if !genuine_rows.is_empty() {
        let mut out = String::with_capacity(genuine_rows.len() * 32 + 32);
        for r in &genuine_rows {
            out.push_str(&format!("0,{},{:.4},\n", r.upc, r.count));
        }
        let mut f = std::fs::File::create(&path)?;
        f.write_all(out.as_bytes())?;
        println!("💾 Wrote count file with {} rows", genuine_rows.len());
        txt_written = true;
    } else {
        println!("ℹ️  No genuine count rows — skipping .txt, generating .qry only");
        txt_written = false;
    }

    // Generate .qry ticket file if any items have tickets
    let ticket_rows: Vec<&SaveRow> = rows.iter().filter(|r| r.has_ticket).collect();
    if !ticket_rows.is_empty() {
        let tickets_dir = std::path::Path::new(output_dir).join("tickets");
        std::fs::create_dir_all(&tickets_dir)?;

        let qry_fname = format!("tickets-{}.qry", timestamp);
        let qry_path = tickets_dir.join(&qry_fname);

        // Check for an existing .qry file in tickets/ to append to
        let existing_qry = std::fs::read_dir(&tickets_dir)?
            .filter_map(|e| e.ok())
            .find(|e| {
                e.path()
                    .extension()
                    .map_or(false, |ext| ext == "qry")
            });

        let mut existing_criteria: Vec<String> = Vec::new();
        if let Some(existing_entry) = existing_qry {
            let existing_path = existing_entry.path();
            if let Ok(content) = std::fs::read_to_string(&existing_path) {
                // Parse out existing [CriteriaN] blocks
                let mut in_criteria = false;
                let mut current_block = String::new();
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("[Criteria") && trimmed.ends_with(']') {
                        // Push previous block if any
                        if in_criteria && !current_block.is_empty() {
                            existing_criteria.push(current_block.trim_end().to_string());
                        }
                        in_criteria = true;
                        current_block = String::new();
                    }
                    if in_criteria {
                        current_block.push_str(line);
                        current_block.push('\n');
                    }
                }
                // Don't forget the last block
                if in_criteria && !current_block.is_empty() {
                    existing_criteria.push(current_block.trim_end().to_string());
                }
                // Remove the old file after reading its criteria
                let _ = std::fs::remove_file(&existing_path);
                println!("🎫 Appending {} existing criteria from {}", existing_criteria.len(), existing_path.display());
            }
        }

        let total_criteria = existing_criteria.len() + ticket_rows.len();

        let mut qry = String::new();
        qry.push_str("[Header]\n");
        qry.push_str("Application=LabelQuery\n");
        qry.push_str("SaveFileVersion=3\n");
        qry.push_str("[Label]\n");
        qry.push_str("Type=1\n");
        qry.push_str(&format!("CriteriaCount={}\n", total_criteria));

        // Write existing criteria blocks first (re-indexed)
        for (i, block) in existing_criteria.iter().enumerate() {
            // Replace the old [CriteriaN] header with new index
            let reindexed = block.replacen(
                &block[..block.find('\n').unwrap_or(block.len())],
                &format!("[Criteria{}]", i),
                1,
            );
            qry.push_str(&reindexed);
            qry.push('\n');
        }

        // Append new criteria blocks
        let criteria_offset = existing_criteria.len();
        for (i, r) in ticket_rows.iter().enumerate() {
            let idx = criteria_offset + i;
            let upc = &r.upc;
            let qty = r.ticket_qty;

            qry.push_str(&format!("[Criteria{}]\n", idx));
            qry.push_str(&format!("Description=UPC equals '{}'\n", upc));
            qry.push_str("BaseQuery=\n");
            qry.push_str(&format!(
                "SQLConditions=i.InActive=0 and ((i.upc = '{}')or (exists(select * from ItemBarcodes ib where (ib.upc=i.upc) and (ib.barcode='{}'))))\n",
                upc, upc
            ));
            qry.push_str("SQLOrderBy=i.SKU\n");
            qry.push_str(&format!("CopiesDesc={}\n", qty));
            qry.push_str(&format!("SQLCopies={}\n", qty));
            qry.push_str("PostPrintAction=0\n");

            if qty != 1 {
                qry.push_str(&format!("CopiesException0Code={}\n", upc));
                qry.push_str(&format!("CopiesException0Value={}\n", qty));
                qry.push_str("CopiesExceptionCount=1\n");
            } else {
                qry.push_str("CopiesExceptionCount=0\n");
            }
        }

        let mut qry_file = std::fs::File::create(&qry_path)?;
        qry_file.write_all(qry.as_bytes())?;
        println!("🎫 Wrote ticket .qry for {} items ({} existing + {} new) to {}", total_criteria, existing_criteria.len(), ticket_rows.len(), qry_path.display());
    }

    Ok((path, txt_written))
}
