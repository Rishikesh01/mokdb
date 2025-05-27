
## ✅ Current Progress
- [x] Parser
- [x] IOManager
- [x] Page + Slot structure
- [ ] Buffer Manager
- [ ] Catalog Manager
- [ ] Table Manager (HeapFile)
- [ ] SQL VM & Execution Engine
- [ ] Join syntax & query planning

---

## 📌 Phase 1: Core Storage & Table Management
> 🧠 **Goal**: Create a minimal database engine that can create tables and perform basic inserts/selects.

### Components
- [x] Parse SQL
- [ ] SQL VM & Execution Engine
  - [ ] Convert AST to Logical Plan (Rust enums/structs)
  - [ ] Implement `Executor` trait with `next()` method
  - [ ] Basic executors: TableScan, Filter, Project
  - [ ] Integration: parse SQL → AST → LogicalPlan 
- [x] Page Layout (slots, metadata)
- [x] IOManager (read/write pages)
- [ ] Buffer Manager
  - [ ] Page replacement (LRU / Clock)
  - [ ] Pin/unpin
  - [ ] Dirty flag tracking
- [ ] Catalog Manager
  - [ ] Table/Column metadata
  - [ ] Persistent storage for catalog
- [ ] Table Manager (HeapFile)
  - [ ] Insert row
  - [ ] Read row
  - [ ] Table scan
  - [ ] Basic filters

### Test Cases
- Create table → Insert rows → Scan table
- Force I/O and eviction (large dataset)

---

## 🔐 Phase 2: ACID & Concurrency
> 🧠 **Goal**: Implement transaction safety, durability, and isolation.

### Atomicity
- [ ] Write-Ahead Logging (WAL)
  - [ ] REDO/UNDO logs
  - [ ] Flush WAL before dirty pages
- [ ] Recovery mechanism
  - [ ] Crash recovery (replay logs)

### Consistency
- [ ] Enforce schema constraints
- [ ] Type validation at insert

### Isolation
- [ ] Lock Manager
  - [ ] Shared/Exclusive locks
  - [ ] Page/row-level locking
  - [ ] Lock table with TTL or deadlock detection

### Durability
- [ ] WAL flushed to disk before commit
- [ ] Checkpointing (optional)

---

## 🌐 Phase 3: Networking & REPL
> 🧠 **Goal**: Make MokDB accessible from the network and support CLI-based interaction.

### Server
- [ ] Binary protocol (optional)
- [ ] Handle multiple connections (threads/goroutines)

### REPL
- [ ] CLI client (`mokdb>`)
- [ ] Basic command history + multiline input

### Client SDK
- [ ] Python SDK
- [ ] JS or Go SDK (stretch goal)

---

## 📈 Phase 4: SQL Execution & Query Planning
> 🧠 **Goal**: Implement query execution engine and support SELECTs, joins, filters.

### SQL VM
- [ ] Design intermediate bytecode/plan
- [ ] Instruction set: Scan, Filter, Project, Join, etc.
- [ ] Execution engine for VM

### Query Planner
- [ ] Parse SELECT queries with JOINs
- [ ] Logical Plan → Physical Plan
- [ ] Rule-based rewrite (e.g. pushdown filters)
- [ ] Cost-based optimizer (optional)

### Operators
- [ ] TableScan
- [ ] Filter
- [ ] Project
- [ ] Nested Loop Join
- [ ] Hash Join (optional)

### Indexing
- [ ] B+ Tree index
- [ ] IndexScan operator
- [ ] Unique index support

---

## ⚡ Phase 5: Distributed Features (Advanced)
> 🧠 **Goal**: Learn basics of distributed databases.

### Sharding
- [ ] Hash-based partitioning
- [ ] Range partitioning

### Replication
- [ ] Leader-follower sync
- [ ] Read replicas

### Coordination
- [ ] Two-Phase Commit (2PC)
- [ ] Raft consensus (simplified)

---

## 🧪 Phase 6: Extras & Stretch Goals
- [ ] Stored Procedures / Triggers
- [ ] MVCC (Multi-Version Concurrency Control)
- [ ] JSON / ARRAY column types
- [ ] Query caching
- [ ] Statistics for cost estimation
- [ ] Materialized indexes
