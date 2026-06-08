# Stocktake-v3 — Barcode Scan Flow

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

## Focus Implementation Notes

- Barcode bar visibility is controlled by the CSS class `.open` on `#barcode-mode-bar`.
- CSS uses `visibility/opacity/max-height` for the show/hide transition, so no inline `display:none` overrides the class state.
- A dedicated `setFocusToBarcodeInput()` helper centralizes focus/clear/select logic for barcode input.
- `toggleBarcodeMode` opens the bar and defers focus via `setTimeout` so the class transition completes first.

---

## Frontend Entry Points

- `toggleBarcodeMode()` — toggles barcode bar visibility and focus
- `resolveScannedBarcode(value)` — lookup and visible-match resolution
- `focusItemForBarcode(upc)` — count-input focus + return-to-barcode behavior
- `itemIsVisible(candidate)` — filter-aware visibility check
- `setFocusToBarcodeInput()` — canonical barcode input focus helper

## Backend Entry Points

- `GET /api/barcode-lookup` (`server.rs`)
- `pool.barcode_lookup_upcs(barcode)` (`db.rs`)

## Build / Backup Reference

- Latest Windows build zip: `~/RussellShared/HermesFiles/HermesOutput/CustomStockTakeApp-V3-B001-<timestamp>.zip`
- Latest project backup: `~/RussellShared/HermesFiles/HermesOutput/stocktake-v3-backup-<timestamp>.zip`
