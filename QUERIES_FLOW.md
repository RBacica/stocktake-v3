# Stocktake-v3 — SQL Queries & Flow Reference

## SQL Queries

### 1. Get Departments
**File:** `src/db.rs` — `get_departments()`
```sql
SELECT ID, Description FROM Departments
WHERE Description NOT IN ('Non Sales', 'EPay')
```

### 2. Get Suppliers (all)
**File:** `src/db.rs` — `get_suppliers()`
```sql
SELECT Code, LastName, FirstName
FROM Customers
WHERE CustType = 'R' AND InActive = '0'
```

### 3. Get Suppliers for Department
**File:** `src/db.rs` — `get_suppliers_for_department()`

**Query 3a — Find supplier codes in department:**
```sql
SELECT DISTINCT i.Supplier
FROM Items i
WHERE i.InActive = '0'
  AND i.Department = '<dept>'
  AND i.Supplier IS NOT NULL
  AND i.Supplier <> ''
ORDER BY i.Supplier
```

**Query 3b — Fetch full supplier details:**
```sql
SELECT Code, LastName, FirstName
FROM Customers
WHERE CustType = 'R'
  AND InActive = '0'
  AND Code IN (<codes_list>)
ORDER BY Code
```

### 4. Refresh Single UPC StockOnHand
**File:** `src/db.rs` — `refresh_upc()`
```sql
SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT) AS StockOnHand
FROM [ItemMovement] m
WHERE m.UPC = '<upc>'
ORDER BY m.ID DESC
```

### 5. Barcode Lookup
**File:** `src/db.rs` — `barcode_lookup_upcs()`
```sql
SELECT UPC FROM ItemBarcodes WHERE barcode = '<barcode>'
```

### 6. Get Sub-Departments
**File:** `src/db.rs` — `get_sub_departments()`
```sql
SELECT [ID], [Description], [DepID]
FROM SubDepartments
WHERE DepID = '<dept>'
```

### 7. Search Items (Main Query)
**File:** `src/db.rs` — `search_items()`
```sql
SELECT
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
    [AND i.Department = '<dept>']
    [AND i.Supplier = '<sup>']
    [AND i.SubDepartment = '<sub_dept>']
ORDER BY i.[Description]
```

### 8. Expand — Fetch Missing Parents
**File:** `src/db.rs` — `search_items()` expansion
```sql
SELECT i.UPC, i.[Description], i.[Department], i.[Supplier],
    (SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT)
     FROM [ItemMovement] m
     WHERE m.UPC = i.UPC ORDER BY m.ID Desc) AS StockOnHand,
    ISNULL(i.ParentUPC, '') AS ParentUPC,
    ISNULL(i.SellingQty, 0) AS SellingQty
FROM Items i
WHERE i.InActive = '0'
  AND i.UPC IN (<parent_upcs>)
ORDER BY i.[Description]
```

### 9. Expand — Fetch Missing Children
**File:** `src/db.rs` — `search_items()` expansion
```sql
SELECT i.UPC, i.[Description], i.[Department], i.[Supplier],
    (SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT)
     FROM [ItemMovement] m
     WHERE m.UPC = i.UPC ORDER BY m.ID Desc) AS StockOnHand,
    ISNULL(i.ParentUPC, '') AS ParentUPC,
    ISNULL(i.SellingQty, 0) AS SellingQty
FROM Items i
WHERE i.InActive = '0'
  AND i.ParentUPC IN (<child_upcs>)
ORDER BY i.[Description]
```

---

## Search Criteria

| Criteria | Type | Required | Description |
|----------|------|----------|-------------|
| Department | Dropdown | Yes | Filter by department. Selecting "All Departments" shows everything. |
| Sub-Department | Dropdown | No | Only visible when "Spirits" department is selected. Filters by sub-department within Spirits. |
| Supplier | Dropdown | No | Filter by supplier. List is filtered based on selected department. |

## Display Formatting

### UPC Column
- Compact format: `nn..nnnnnn` (first 2 digits + `..` + last 6 digits)
- If UPC ≤ 8 characters: displayed as-is

### StockOnHand Column
- Whole numbers: `475` (no decimal)
- With decimals: `12.50`

### Variance Column
- `|variance| < 0.03` → `0.00` (no prefix, grey)
- Positive → `+nnn.nn` (green)
- Negative → `-nnn.nn` (red)
- Parent combined: same format with `Total` label below the number

---

## Flow Breakdown

### Startup Flow
1. **Frontend loads** → `DOMContentLoaded` fires
2. **Load departments** → `GET /api/departments` → populates department dropdown
3. **Load suppliers** → `GET /api/suppliers` → populates supplier dropdown
4. **Sub-department dropdown** hidden by default

### Search Flow
1. User selects **Department** → `onDepartmentChange()`:
   - `loadSuppliersForDepartment(dept)` → `GET /api/suppliers-for-dept?department=<id>` → filters supplier dropdown
   - `loadSubDepartments(dept)` → `GET /api/sub-departments?department=<id>` → populates sub-department dropdown (only shown when "Spirits" department is selected)
2. User selects **Sub-Department** (optional, only visible for Spirits)
3. User clicks **Search** → `searchItems()`:
   - Builds URL: `/api/search?department=X&supplier=Y&sub_department=Z`
   - `GET /api/search` → `search_items()` handler
4. **Backend search** (`search_items()` in `db.rs`):
   - Runs main filtered query (Query 7)
   - Collects parent UPCs needed (from matched children)
   - Collects child UPCs needed (from matched parents)
   - Runs expansion query for missing parents (Query 8)
   - Runs expansion query for missing children (Query 9)
   - Returns combined `Vec<StockItem>` as JSON
5. **Frontend receives results**:
   - `buildParentChildMap()` — builds `childrenByParent` and `childUpcs` maps
   - `renderTable()` — renders parent rows + nested child rows
   - Variance computed: `|variance| < 0.03` → displays `0.00`; otherwise `+nnn.nn` / `-nnn.nn`
   - SOH displayed: whole number if no decimals, otherwise `nn.nn`
   - UPC displayed: compact `nn..nnnnnn` format

### Count Entry Flow
1. User enters count in input → `onCountChange(upc)`:
   - Persists to `counts[upc]`
   - Computes variance: `count - stock_on_hand`
   - If `|variance| < 0.03` → displays `0.00` (no prefix, no highlight)
   - If `|variance| ≥ 0.03` → displays `+nnn.nn` or `-nnn.nn` with highlight
   - Updates parent combined variance if item is a child
   - Updates standalone variance if item is standalone

### Refresh Flow
1. **Single item refresh** → `GET /api/refresh-upc?upc=<upc>` → Query 4
2. **Refresh all** → parallel calls to `GET /api/refresh-upc?upc=<upc>` for each item
3. Updates SOH display (whole number or `nn.nn`)

### Barcode Scan Flow
1. Scan → `resolveScannedBarcode(value)`:
   - `GET /api/barcode-lookup?barcode=<value>` → Query 5
   - Matches UPCs against visible items
   - Focuses matching count input
   - After count entry, returns focus to barcode input

### Save Flow
1. **Save** → `POST /api/save` with counted rows
2. Writes `stocktake-<timestamp>.txt` file (format: `0,<UPC>,<Count>,`)
3. Optionally writes `tickets-<timestamp>.qry` for ticketed items

### Reset Flow
1. **Reset** → `resetAll()`:
   - Turns off Barcode Mode
   - Clears all state (`counts`, `tickets`, `childrenByParent`, etc.)
   - Resets `lastFilter` to `{ dept: 'ALL', sup: 'ALL', subDep: 'ALL' }`
   - Resets sub-department dropdown to `ALL` and hides it
   - Shows empty state in results table

---

## API Endpoints Summary

| Endpoint | Method | Handler | Description |
|----------|--------|---------|-------------|
| `/api/departments` | GET | `get_departments` | List all departments |
| `/api/suppliers` | GET | `get_suppliers` | List all suppliers |
| `/api/suppliers-for-dept` | GET | `get_suppliers_for_dept` | Suppliers filtered by department |
| `/api/sub-departments` | GET | `get_sub_departments` | Sub-departments for a department |
| `/api/search` | GET | `search_items` | Search items by dept/supplier/sub-dept |
| `/api/refresh-upc` | GET | `refresh_upc` | Refresh SOH for single UPC |
| `/api/barcode-lookup` | GET | `barcode_lookup` | Lookup UPCs by barcode |
| `/api/save` | POST | `save_counts` | Save stock take results |

---

## File Reference

| File | Purpose |
|------|---------|
| `src/db.rs` | All database queries, connection pool, data structures |
| `src/server.rs` | Actix-web route handlers, API endpoint wiring |
| `web/index.html` | Full frontend — UI, JS state, rendering, event handling |
