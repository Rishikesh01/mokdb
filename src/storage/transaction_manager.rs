use std::{
    collections::{HashSet, VecDeque},
    sync::{atomic::AtomicU64, Arc},
};

use dashmap::DashMap;
use parking_lot::{Condvar, Mutex};

use crate::{errors::MokErrors, storage::wait_graph::WaitGraph};

#[derive(Eq, PartialEq, Hash, Clone)]
pub enum ResourceId {
    Index(String),
    Table(String),
    Page {
        table: String,
        page: u64,
    },
    Tuple {
        table: String,
        page: u64,
        tuple: u64,
    },
}

#[derive(Clone, Copy)]
pub enum LockType {
    ReadIntent,
    WriteIntent,
    ReadIntentWithWriteUpgrade,

    ReadLock, // Shared
    Write,    // Exclusive
}

impl LockType {
    pub fn compatible_with(&self, other: &LockType) -> bool {
        use LockType::*;

        match (self, other) {
            (ReadIntent, ReadIntent) => true,
            (ReadIntent, WriteIntent) => true,
            (ReadIntent, ReadLock) => true,
            (ReadIntent, ReadIntentWithWriteUpgrade) => true,
            (ReadIntent, Write) => false,

            (WriteIntent, ReadIntent) => true,
            (WriteIntent, WriteIntent) => true,
            (WriteIntent, ReadLock) => false,
            (WriteIntent, ReadIntentWithWriteUpgrade) => false,
            (WriteIntent, Write) => false,

            (ReadLock, ReadIntent) => true,
            (ReadLock, ReadLock) => true,
            (ReadLock, WriteIntent) => false,
            (ReadLock, ReadIntentWithWriteUpgrade) => false,
            (ReadLock, Write) => false,

            (ReadIntentWithWriteUpgrade, ReadIntent) => true,
            (ReadIntentWithWriteUpgrade, ReadLock) => false,
            (ReadIntentWithWriteUpgrade, WriteIntent) => false,
            (ReadIntentWithWriteUpgrade, ReadIntentWithWriteUpgrade) => false,
            (ReadIntentWithWriteUpgrade, Write) => false,

            (Write, _) => false,
            (_, Write) => false,
        }
    }
}

struct Waiters {
    transaction_id: u64,
    lock_type: LockType,
}

struct LockState {
    lock_type: Option<LockType>,
    holders: HashSet<u64>,
    waiters: VecDeque<Waiters>,
}

struct Lock {
    state: Mutex<LockState>,
    cond: Condvar,
}

impl ResourceId {
    pub fn ancestors(&self) -> Vec<ResourceId> {
        match self {
            ResourceId::Index(_) => {
                // Index has no parents
                vec![]
            }

            ResourceId::Table(_) => {
                // Table has no parents
                vec![]
            }

            ResourceId::Page { table, .. } => {
                vec![ResourceId::Table(table.clone())]
            }

            ResourceId::Tuple { table, page, .. } => {
                vec![
                    ResourceId::Page {
                        table: table.clone(),
                        page: *page,
                    },
                    ResourceId::Table(table.clone()),
                ]
            }
        }
    }
}

impl LockState {
    fn new() -> Self {
        Self {
            lock_type: None,
            holders: HashSet::new(),
            waiters: VecDeque::new(),
        }
    }
}

impl Default for Lock {
    fn default() -> Self {
        Self {
            state: Mutex::new(LockState::new()),
            cond: Condvar::default(),
        }
    }
}

pub struct TransactionManager {
    next_tx_id: AtomicU64,
    table_level_locks: DashMap<ResourceId, Arc<Lock>>,
    wait_graph: WaitGraph,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            next_tx_id: AtomicU64::new(1),
            table_level_locks: DashMap::new(),
            wait_graph: WaitGraph::new(),
        }
    }

    fn paths(&self, resource_id: ResourceId) -> Vec<ResourceId> {
        match resource_id {
            ResourceId::Index(e) => vec![ResourceId::Index(e)],
            ResourceId::Table(e) => vec![ResourceId::Table(e)],
            ResourceId::Page { table, page } => {
                vec![
                    ResourceId::Table(table.clone()),
                    ResourceId::Page { table, page },
                ]
            }
            ResourceId::Tuple { table, page, tuple } => vec![
                ResourceId::Table(table.clone()),
                ResourceId::Page {
                    table: table.clone(),
                    page: page,
                },
                ResourceId::Tuple { table, page, tuple },
            ],
        }
    }

    fn acquire_current_level(
        &self,
        tx_id: u64,
        resource_id: ResourceId,
        requested: LockType,
    ) -> Result<(), MokErrors> {
        for path in self.paths(resource_id) {
            loop {
                let lock = self
                    .table_level_locks
                    .entry(path.clone())
                    .or_insert(Arc::new(Lock::default()))
                    .clone();

                let mut state = lock.state.lock();
                match state.lock_type {
                    None => {
                        state.lock_type = Some(requested);
                        state.holders.insert(tx_id);
                        return Ok(());
                    }

                    Some(held_type) => {
                        // If this tx already holds the lock → check upgrade possibility
                        if state.holders.contains(&tx_id) {
                            if requested.compatible_with(&held_type) {
                                // Already holds with compatible lock → nothing to do
                                return Ok(());
                            }

                            // UPGRADE
                            // But only allowed if this tx is the sole holder
                            if state.holders.len() == 1 {
                                state.lock_type = Some(requested);
                                return Ok(());
                            }

                            // Other transactions holding lock → must wait
                            for holders in state.holders.iter() {
                                self.wait_graph.add_edge(tx_id, *holders);
                            }
                        }

                        // Conflicting lock
                        if !requested.compatible_with(&held_type) {
                            // enqueue and wait
                            state.waiters.push_back(Waiters {
                                transaction_id: tx_id,
                                lock_type: requested,
                            });

                            // Wait until woken and retry
                            lock.cond.wait(&mut state);
                            continue;
                        }

                        // Compatible shared lock: allow multiple holders
                        state.holders.insert(tx_id);
                        return Ok(());
                    }
                }
            }
        }

        return Ok(());
    }

    pub fn begin(self) -> u64 {
        self.next_tx_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /*
     *  We will first check if an lock request is present
     *  when lock request is not present we will look at higher level conflicting locks i.e
     *  write lock on entire table will end up rejecting write or read lock on a page
     *
     *  if lock is present then we will check if our lock request is compatible with existing
     *  lock type i.e readers being compatible with each other
     *  if our lock request is not compatible then we will wait for conditional var to trigger and
     *  loop through all the tx queue and give access to one or more transactions
     *
     *
     */
    pub fn aquire(
        &mut self,
        tx_id: u64,
        resource_id: ResourceId,
        lock_type: LockType,
    ) -> Result<(), MokErrors> {
        return self.acquire_current_level(tx_id, resource_id, lock_type);
    }
}
