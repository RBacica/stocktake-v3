# Stocktake-v3 — SQL Queries & Barcode Scan Flow

## Current SQL Queries

### 1. Barcode Lookup
File: `src/db.rs`

```sql
SELECT UPC FROM ItemBarcodes WHERE barcode = '<safe_barcode>'
```

- Input is sanitized by replacing single quotes with doubled single quotes.
- Returns `Vec<String>` of matched UPCs.

### 2. Main Item Search
File: `src/db.rs`

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
    -- optional department clause
    -- optional supplier clause
ORDER BY i.[Description]
```

### 3. Single-Item Stock Refresh
File: `src/db.rs`

```sql
SELECT TOP 1 CAST((m.QtyOnHand + m.Quantity) AS FLOAT) AS StockOnHand
FROM [ItemMovement] m
WHERE m.UPC = '<upc>'
ORDER BY m.ID DESC
```

---

## Barcode Scan Handling Flow

1. **Scan** → Frontend calls `resolveScannedBarcode(value)` with raw barcode string.
2. **Frontend** → strips to `raw` and calls:
   `GET /api/barcode-lookup?barcode=' + encodeURIComponent(raw)`
3. **Backend (`server.rs`)** → `barcode_lookup()`:
   - trims barcode
   - rejects empty input with `400`
   - calls `pool.barcode_lookup_upcs(barcode)`
4. **DB (`db.rs`)** → runs barcode lookup SQL and returns `Vec<String>` of matched UPCs.
5. **Backend** → returns `200` with JSON:
   ```json
   { "upcs": ["MATCHED_UPC_1", "MATCHED_UPC_2", ...] }
   ```
   or `500` on DB error.
6. **Frontend** → processes result:
   - if empty → falls back to raw scanned barcode as UPC
   - iterates matched UPCs and checks `itemIsVisible(candidate)` against current filtered items
   - first visible match → `focusItemForBarcode(upc)`
   - none match → **"Barcode not found in current view"**
7. **Focus behavior**:
   - **Barcode Mode toggle** → focus moves to barcode input
   - **Scan/lookup success** → focus moves to matching count input
   - **After count entered** → focus returns to barcode input
   - **Reset** → Barcode Mode is turned off and hidden
8. **Multi-barcode handling**: all UPCs for the scanned barcode are returned; first visible match wins.

---

## Frontend Entry Points

- `toggleBarcodeMode()` — toggles barcode bar visibility and focus
- `resolveScannedBarcode(value)` — lookup and visible-match resolution
- `focusItemForBarcode(upc)` — count-input focus + return-to-barcode behavior
- `itemIsVisible(candidate)` — filter-aware visibility check

## Backend Entry Points

- `GET /api/barcode-lookup` (`server.rs`)
- `pool.barcode_lookup_upcs(barcode)` (`db.rs`)
