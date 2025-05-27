use std::{
    collections::{
        hash_map::{DefaultHasher, Entry},
        HashMap, HashSet,
    },
    hash::{Hash, Hasher},
    sync::Arc,
};

use tokio::sync::{Mutex, Notify};

use super::wait_graph::{TransactionLockEdge, WaitGraph};

const NUM_SHARDS: usize = 64;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum LockType {
    WillTakeSharedReadLock = 0,
    WillTakeWriteLock = 1,
    HasReadLock = 2,
    HasWriteLock = 3,
}

impl LockType {
    pub fn compatibility_check(self, current_lock_type: LockType) -> bool {
        const COMPATIBILITY_MATRIX: [[bool; 4]; 4] = [
            // WTSR, WTWL, HRL, HWL
            [true, true, true, false],    // WTSR
            [true, true, false, false],   // WTWL
            [true, false, false, false],  // HRL
            [false, false, false, false], // HWL
        ];

        let self_index = self as usize;
        let current_index = current_lock_type as usize;
        COMPATIBILITY_MATRIX[self_index][current_index]
    }
}

#[derive(Clone, Debug)]
pub struct Waiter {
    pub tx_id: u64,
    pub lock_type: LockType,
    pub notify: Arc<Notify>,
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub enum Lock {
    Table(u64),
    Page(u64, u64),
    Tuple(u64, u64, u64),
}

pub struct LockInfo {
    lock_type: Option<LockType>,
    concurrent_tx: HashSet<u64>,
    waiters: HashMap<u64, Waiter>,
}

pub struct LockManager {
    lock_shards: Arc<Vec<Mutex<HashMap<Lock, LockInfo>>>>,
    wait_graph: Arc<WaitGraph>,
}

impl LockManager {
    pub fn new(wait_graph: WaitGraph) -> Self {
        let mut shards = Vec::with_capacity(NUM_SHARDS);
        for _ in 0..NUM_SHARDS {
            shards.push(Mutex::new(HashMap::new()));
        }
        let graph = Arc::new(wait_graph);
        Self {
            wait_graph: graph,
            lock_shards: Arc::new(shards),
        }
    }

    fn get_shard_index(lock: &Lock) -> usize {
        let mut hasher = DefaultHasher::new();
        lock.hash(&mut hasher);
        (hasher.finish() as usize) % NUM_SHARDS
    }

    fn get_shard(&self, lock: &Lock) -> &Mutex<HashMap<Lock, LockInfo>> {
        &self.lock_shards[Self::get_shard_index(lock)]
    }

    pub async fn acquire_lock(&self, tx_id: u64, lock: Lock, lock_type: LockType) {
        let notify = Arc::new(Notify::new());

        loop {
            let shard = self.get_shard(&lock);
            let mut map = shard.lock().await;

            let entry = map.entry(lock).or_insert_with(|| LockInfo {
                lock_type: None,
                concurrent_tx: HashSet::new(),
                waiters: HashMap::new(),
            });

            {
                if entry.concurrent_tx.contains(&tx_id) {
                    return;
                }
            }

            if entry.lock_type.is_none() {
                entry.lock_type = Some(lock_type);
                entry.concurrent_tx.insert(tx_id);
                return;
            }

            if entry
                .lock_type
                .is_some_and(|lt| lt.compatibility_check(lock_type))
            {
                entry.concurrent_tx.insert(tx_id);
                entry.lock_type = Some(lock_type);
                return;
            }

            if let Entry::Vacant(e) = entry.waiters.entry(tx_id) {
                let waiter = Waiter {
                    tx_id,
                    lock_type,
                    notify: notify.clone(),
                };

                for current_tx in entry.concurrent_tx.iter() {
                    self.wait_graph.add_edge(tx_id, *current_tx).await;
                }

                e.insert(waiter.clone());
            }
            notify.notified().await;
        }
    }

    pub async fn release_lock(&self, tx_id: u64, lock: Lock) {
        let shard = self.get_shard(&lock);
        let mut map = shard.lock().await;

        if let Some(entry) = map.get_mut(&lock) {
            let mut holders = entry.concurrent_tx.clone();
            holders.remove(&tx_id);

            if holders.is_empty() {
                entry.lock_type = None;

                for waiter in entry.waiters.values() {
                    waiter.notify.notify_one();
                }
            }
        }
    }
}
