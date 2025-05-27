# Database Engine Development - Next Steps (Detailed)

---

## 1. Page Manager (Disk I/O Layer)

**Purpose**: Handle low-level reading and writing of pages to/from disk.

- Load pages by `page_id` from the file
- Allocate new pages, managing the next available page ID
- Write pages back to disk after modifications

**Key methods**:
- `read_page(id: u64) -> Page`
- `write_page(page: Page)`
- `allocate_page() -> u64`

---

## 2. Buffer Pool (Page Cache)

**Purpose**: Cache pages in memory to reduce disk I/O, improve performance, and ensure data correctness.

Responsibilities:
- Keep a fixed-size map of pages in memory
- Track which pages are dirty (modified but not yet flushed)
- Use an eviction policy (like LRU) to remove pages when the cache is full
- Provide methods to fetch pages, mark them dirty, and flush changes to disk

**Suggested API**:
- `get_page(page_id: u64) -> &mut Page`
- `mark_dirty(page_id: u64)`
- `flush(page_id: u64)`
- `flush_all()`

---

## 3. HeapFile (Tuple Table Heap)

**Purpose**: Manage a collection of pages for a single logical table.

- Insert tuples by finding or allocating pages with free space
- Scan all pages for queries
- Interface with the Buffer Pool to read/write pages safely

Maintain a list of page IDs belonging to the table and use Buffer Pool to access them.

---

## 4. Catalog (System Metadata Tables)

**Purpose**: Maintain metadata about user-defined tables, their columns, and indexes.

Common system tables:
- `tables` — with columns like `table_id`, `name`, and `root_page_id`
- `columns` — describing columns per table: `table_id`, `column_name`, `type`, `position`
- `indexes` — index info per table and column

Catalog API examples:
- `create_table(name, columns)`
- `get_table_schema(name) -> Vec<ColumnInfo>`
- `get_table_root_page(name) -> u64`

---

## 5. Query Execution (Basic Engine)

**Purpose**: Execute basic queries such as `SELECT` and `INSERT`.

- Scan tuples from HeapFile respecting MVCC visibility
- Deserialize tuples into usable values
- Filter and project data as per queries

Start with simple queries like:

```sql
SELECT * FROM table;
INSERT INTO table VALUES (...);

