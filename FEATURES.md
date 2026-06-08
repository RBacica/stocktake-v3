# Stocktake-v3 — Program Features & Usage Guide

## Overview

Stocktake-v3 is a Windows desktop application for performing inventory stock
counts. It connects to an MS SQL Server database (Infinity Back Office) to look
up items, then provides a browser-based UI where users enter physical counts.

When a stocktake is saved, the application produces **two output files**:

| File | Format | Purpose | Consumed By |
|------|--------|---------|-------------|
| Count file | `.txt` (CSV-like) | Records item counts and variances for import into the stocktake program | **Infinity Stocktake** (native stocktake program) |
| Ticket file | `.qry` (Label Query) | Generates price-ticket/label print jobs for selected items | **Infinity Labels** (label-printing program) |

---

## The Count File (`.txt`)

### What It Is

A plain-text CSV file containing one row per counted item. This file is loaded
into the **Infinity** stocktake program to reconcile counted quantities against
the system's recorded stock-on-hand.

### Location

```
<output_dir>/stocktake-YYYY-MM-DD-HH-MM-SS.txt
```

`output_dir` defaults to a `stocktake_output/` folder created next to
`stocktake-v3.exe`. It can be changed via `output_dir` under `[server]` in
`config.toml`.

### File Format

Each line has three comma-separated fields:

```
0,<UPC>,<COUNT>
```

| Field | Meaning | Example |
|-------|---------|---------|
| `0` | Record type identifier (always `0` for stocktake rows) | `0` |
| `<UPC>` | The item's barcode / UPC | `9421033031648` |
| `<COUNT>` | The physical count entered by the user, formatted to 4 decimal places | `12.0000` |

**Example — 3 counted items:**

```csv
0,9421033031648,12.0000
0,080686010111,5.0000
0,080686015406,0.0000
```

### How It Gets Loaded into Infinity

1. In the **Infinity** stocktake program, use the count-file import function to
   open the `.txt` file.
2. Each line's UPC is matched to the corresponding item in the Infinity
   database.
3. The count value replaces or updates the expected quantity for that item.

### When the `.txt` Is NOT Written

If the user only toggled **Ticket** on items but entered no actual counts (and
has "Treat no count as zero" turned OFF), the `.txt` is **skipped** — only the
`.qry` ticket file is generated. This avoids creating a count file full of
zeros when the user only wants to print labels.

---

## The Ticket File (`.qry`)

### What It Is

A Label Query file that defines which items need price tickets or barcode labels
printed. This file is loaded into the **Infinity Labels** program (or any
LabelQuery-compatible label-printing software).

### Location

```
<output_dir>/tickets/tickets-YYYYMMDD_HHMMSS.qry
```

The `tickets/` subdirectory is created automatically inside `output_dir`.

### File Format

`.qry` files use an INI-style format. Each ticketed item gets a `[CriteriaN]`
block containing the UPC, the SQL query to find the item in the database, and
the number of labels to print.

**Minimal example — 2 items, default 1 label each:**

```ini
[Header]
Application=LabelQuery
SaveFileVersion=3
[Label]
Type=1
CriteriaCount=2
[Criteria0]
Description=UPC equals '9421033031648'
BaseQuery=
SQLConditions=i.InActive=0 and ((i.upc = '9421033031648')or (exists(select * from ItemBarcodes ib where (ib.upc=i.upc) and (ib.barcode='9421033031648'))))
SQLOrderBy=i.SKU
CopiesDesc=1
SQLCopies=1
PostPrintAction=0
CopiesExceptionCount=0
[Criteria1]
Description=UPC equals '080686010111'
BaseQuery=
SQLConditions=i.InActive=0 and ((i.upc = '080686010111')or (exists(select * from ItemBarcodes ib where (ib.upc=i.upc) and (ib.barcode='080686010111'))))
SQLOrderBy=i.SKU
CopiesDesc=1
SQLCopies=1
PostPrintAction=0
CopiesExceptionCount=0
```

### Key Fields

| Field | Description |
|-------|-------------|
| `CriteriaCount` | Total number of `[CriteriaN]` blocks in the file |
| `Description` | Human-readable label showing the UPC filter |
| `SQLConditions` | Database query that matches the item by primary UPC or alternate barcode |
| `CopiesDesc` / `SQLCopies` | Number of labels to print for this item |
| `CopiesExceptionCount` | `0` when printing 1 label; `1` when printing more than 1 |
| `CopiesException0Code` | *(only when qty > 1)* The actual UPC value |
| `CopiesException0Value` | *(only when qty > 1)* The number of labels to print |

### Multiple Labels (ticket qty > 1)

When a user sets a ticket quantity greater than 1, exception lines are added
before `CopiesExceptionCount`. Example for 3 labels:

```ini
CopiesDesc=3
SQLCopies=3
PostPrintAction=0
CopiesException0Code=9421033031648
CopiesException0Value=3
CopiesExceptionCount=1
```

### Accumulate Across Saves

The `.qry` file uses **append behavior**: if a `.qry` file already exists in the
`tickets/` folder, the old criteria blocks are preserved, re-indexed, and the
new items are appended. The result is a single `.qry` file containing all
ticketed items from all saves.

**Example — two successive saves:**

- **Save 1**: tickets for UPC-A, UPC-B → `tickets-20260601_120000.qry` with `CriteriaCount=2`
- **Save 2**: tickets for UPC-C → deletes old file, creates `tickets-20260601_130000.qry` with `CriteriaCount=3`
  - `[Criteria0]` = UPC-A (carried from save 1)
  - `[Criteria1]` = UPC-B (carried from save 1)
  - `[Criteria2]` = UPC-C (new in save 2)

The filename always reflects the latest save timestamp.

### How It Gets Loaded into Infinity Labels

1. Open the **Infinity Labels** program.
2. Use the import/open function to load the `.qry` file.
3. The program reads each `[CriteriaN]` block to determine which items to print
   labels for and how many copies.
4. Execute the print job.

---

## How to Use — End-to-End Workflow

### 1. Search for Items

- Select a **Department** from the dropdown.
- If the selected department is **Spirits**, a **Sub-Department** dropdown
  appears below it — optionally select a sub-department to further filter.
- Select a **Supplier** from the dropdown (filtered by selected department).
- Click **Search**.
- The app queries the database and displays matching active items.
- **Parent items** automatically include their children (e.g. 6-packs, 12-packs)
  even if the children belong to a different department or supplier.

### 2. Enter Counts

- Type the physical count into the **Count** input for each item.
- The **Variance** column updates live.
- For parent items, variance is **combined**: parent count + (child count ×
  child's selling quantity) − parent stock_on_hand.
- Variance display: `+nnn.nn` / `-nnn.nn` with sign prefix; values within
  ±0.03 display as `0.00`.

### 3. Mark Items for Ticket Printing (Optional)

- Click the **Ticket** button on any row to toggle it on (highlights purple).
- A quantity input appears — set how many labels to print (default: 1,
  range: 1–999).
- Ticketed items always generate a `.qry` file on save, even if no count was
  entered.

### 4. Refresh Stock On Hand (Optional)

- Click **Refresh All** to re-fetch SOH for every displayed item from the
  database.
- Click the **↻** button on a single row to refresh just that item (and its
  children, if any).

### 5. Save Results

- Click **Export**.
- The app writes the output files to the `stocktake_output/` folder.
- A confirmation dialog shows:
  - For counts: the `.txt` filename and number of rows saved.
  - For tickets: the `.qry` filename in the `tickets/` subfolder.

### 6. Load Files into Infinity

- **Count file** → Open in **Infinity** stocktake program via count-file import.
- **Ticket file** → Open in **Infinity Labels** program to print price tickets /
  barcode labels.

---

## Search Criteria

| Criteria | Type | Required | Description |
|----------|------|----------|-------------|
| Department | Dropdown | Yes | Filter by department. Selecting "All Departments" shows everything. |
| Sub-Department | Dropdown | No | Only visible when "Spirits" department is selected. Filters by sub-department within Spirits. |
| Supplier | Dropdown | No | Filter by supplier. List is filtered based on selected department. |

---

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

### Parent/Child Grouping
- Parent rows display combined variance (parent + all children contribution)
- Child rows are nested directly under their parent
- Children are hidden from top-level iteration
- Filters (variance-only, ticketed-only) inherit from parent to children

---

## Barcode Mode

- Toggle **Barcode Mode** to enable barcode scanning.
- When enabled, a barcode input bar appears at the top of the results.
- Scanning a barcode looks up the item and focuses its count input.
- After entering a count, focus returns to the barcode input for the next scan.
- Barcode Mode is turned off when Reset is clicked.

---

## Output Folder Structure

```
stocktake-v3.exe
config.toml
web/
stocktake_output/
├── stocktake-2026-06-01-12-00-00.txt     ← Count file (→ Infinity Stocktake)
├── stocktake-2026-06-01-13-30-00.txt
└── tickets/
    └── tickets-20260601_130000.qry        ← Ticket file (→ Infinity Labels)
```

---

## Summary

| Action in Stocktake-v3 | Output File | Load Into |
|------------------------|-------------|-----------|
| Enter counts + Export  | `.txt` count file | Infinity Stocktake program |
| Toggle Ticket + Export | `.qry` ticket file (in `tickets/` subfolder) | Infinity Labels program |
| Both counts + tickets  | Both files generated in the same save | Both programs |

---

## Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Console flashes and closes | Startup error | Run from a terminal to see the error message |
| "config.toml exists but could not be parsed" | Unescaped backslash in config | Double backslashes or use single quotes |
| "connection_string is empty" | Config not filled in | Edit `config.toml` with your SQL Server details |
| "Could not bind to..." | Port already in use | Change `port` in `config.toml` |
| "Database error" in UI | SQL Server unreachable | Check connection string and network |
| Sub-department dropdown not showing | Only visible for "Spirits" department | Select Spirits department first |
