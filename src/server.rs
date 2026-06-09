use actix_web::{web, HttpResponse};

use crate::db;

/// Output directory for saved stock takes, shared with handlers as app data.
#[derive(Clone)]
pub struct OutputDir(pub String);

/// Endpoints: /api/departments, /api/suppliers, /api/suppliers-for-dept,
/// /api/search, /api/save, /api/refresh-upc, /api/barcode-lookup,
/// /api/sub-departments. Static files served from `web/` with `index.html`
/// as the default file; API routes are registered first so they take precedence.
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/api/departments").route(web::get().to(get_departments)))
        .service(web::resource("/api/suppliers").route(web::get().to(get_suppliers)))
        .service(web::resource("/api/suppliers-for-dept").route(web::get().to(get_suppliers_for_dept)))
        .service(web::resource("/api/search").route(web::get().to(search_items)))
        .service(web::resource("/api/save").route(web::post().to(save_counts)))
        .service(web::resource("/api/refresh-upc").route(web::get().to(refresh_upc)))
        .service(web::resource("/api/barcode-lookup").route(web::get().to(barcode_lookup)))
        .service(web::resource("/api/sub-departments").route(web::get().to(get_sub_departments)));
}

// ─────────────────────────────────────────────
//  API Handlers
// ─────────────────────────────────────────────

// GET /api/departments
// 200 deps | 500 DB down
async fn get_departments(pool: web::Data<db::DbPool>) -> HttpResponse {
    // Always returns a populated list: live DB if reachable, otherwise the
    // fixed Infinity department list. The UI dropdown is never empty.
    HttpResponse::Ok().json(pool.departments_or_fallback().await)
}

// GET /api/suppliers
// 200 list | 500 DB down
async fn get_suppliers(pool: web::Data<db::DbPool>) -> HttpResponse {
    // Always returns a populated list: live DB if reachable, otherwise the
    // four known suppliers (code + name).
    HttpResponse::Ok().json(pool.suppliers_or_fallback().await)
}

// GET /api/suppliers-for-dept?department=<id>
// 200 list | 500 DB | 400 missing/invalid
/// Returns only suppliers that have items in the given department.
/// If department=ALL, returns all suppliers.
async fn get_suppliers_for_dept(
    pool: web::Data<db::DbPool>,
    query: web::Query<db::DeptQuery>,
) -> HttpResponse {
    let dept = query.department.clone().unwrap_or_else(|| "ALL".to_string());
    match pool.get_suppliers_for_department(&dept).await {
        Ok(list) => HttpResponse::Ok().json(list),
        Err(e) => {
            eprintln!("Failed to get suppliers for dept {}: {}", dept, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// GET /api/search?department=&supplier=&sub_department=
// 200 items | 500 DB
async fn search_items(
    pool: web::Data<db::DbPool>,
    query: web::Query<db::SearchQuery>,
) -> HttpResponse {
    let dept = query.department.clone().unwrap_or_else(|| "ALL".to_string());
    let sup = query.supplier.clone().unwrap_or_else(|| "ALL".to_string());
    let sub_dept = query.sub_department.clone().unwrap_or_else(|| "ALL".to_string());

    match pool.search_items(&dept, &sup, &sub_dept).await {
        Ok(items) => HttpResponse::Ok().json(items),
        Err(e) => {
            eprintln!("Failed to search items: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// GET /api/sub-departments?department=<id>
// 200 list | 500 DB
/// Returns sub-departments for the given department.
async fn get_sub_departments(
    pool: web::Data<db::DbPool>,
    query: web::Query<db::SubDeptQuery>,
) -> HttpResponse {
    let dept = query.department.clone().unwrap_or_default();
    match pool.get_sub_departments(&dept).await {
        Ok(subs) => HttpResponse::Ok().json(subs),
        Err(e) => {
            eprintln!("Failed to get sub-departments for dept {}: {}", dept, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// GET /api/refresh-upc?upc=<upc>
// 200 {upc, stock_on_hand} | 400 missing | 500 DB
/// Refresh StockOnHand for a single UPC.
/// Query param: `upc`
/// Returns JSON: { "upc": "...", "stock_on_hand": <i64> }
async fn refresh_upc(
    pool: web::Data<db::DbPool>,
    query: web::Query<db::UpcQuery>,
) -> HttpResponse {
    let upc = query.upc.clone().unwrap_or_default();
    if upc.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Missing upc query parameter"
        }));
    }

    match pool.refresh_upc(&upc).await {
        Ok(stock_on_hand) => HttpResponse::Ok().json(serde_json::json!({
            "upc": upc,
            "stock_on_hand": stock_on_hand
        })),
        Err(e) => {
            eprintln!("Failed to refresh upc {}: {}", upc, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// GET /api/barcode-lookup?barcode=<barcode>
// 200 {upcs} | 400 missing | 500 DB
/// Barcode lookup: return all UPCs for a scanned barcode from ItemBarcodes.
/// Query param: `barcode`
/// Returns JSON: { "upcs": ["...", "..."] }
async fn barcode_lookup(
    pool: web::Data<db::DbPool>,
    query: web::Query<db::BarcodeQuery>,
) -> HttpResponse {
    let barcode = query.barcode.trim();
    if barcode.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Missing barcode query parameter"
        }));
    }

    match pool.barcode_lookup_upcs(barcode).await {
        Ok(upcs) => HttpResponse::Ok().json(serde_json::json!({
            "upcs": upcs
        })),
        Err(e) => {
            eprintln!("Failed barcode lookup for {}: {}", barcode, e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Database error: {}", e)
            }))
        }
    }
}

// POST /api/save — 200 ok | 400 empty | 500 write fail
/// Persist counted rows to a timestamped .txt file; generate a .qry ticket
/// file when any row has has_ticket=true. The browser clears its view on 200.
async fn save_counts(
    output_dir: web::Data<OutputDir>,
    body: web::Json<db::SaveRequest>,
) -> HttpResponse {
    let req = body.into_inner();

    if req.rows.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "status": "error",
            "message": "No rows to save"
        }));
    }

    let count = req.rows.len();
    match db::save_stocktake(&output_dir.0, &req.rows) {
        Ok((path, txt_written)) => {
            let path_str = path.display().to_string();
            println!("💾 Saved {} rows to {}", count, path_str);
            HttpResponse::Ok().json(serde_json::json!({
                "status": "ok",
                "message": format!("Saved {} rows", count),
                "rows": count,
                "file": if txt_written { Some(path_str) } else { None },
                "txt_written": txt_written
            }))
        }
        Err(e) => {
            eprintln!("Failed to save stock take: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "status": "error",
                "message": format!("Failed to save: {}", e)
            }))
        }
    }
}
