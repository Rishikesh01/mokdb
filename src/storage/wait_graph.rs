use std::collections::{HashMap, HashSet};

use tokio::sync::Mutex;

use super::lock_manager::{Lock, LockType};

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct TransactionLockEdge {
    pub lock: Lock,
    pub lock_type: LockType,
}

pub struct WaitGraph {
    graph: Mutex<HashMap<u64, HashSet<u64>>>,
}

impl WaitGraph {
    pub fn new() -> Self {
        Self {
            graph: Mutex::new(HashMap::new()),
        }
    }

    pub async fn add_node(&mut self, tx_id: u64) {
        self.graph
            .lock()
            .await
            .entry(tx_id)
            .or_insert_with(HashSet::new);
    }

    pub async fn add_edge(&self, tx_id: u64, dep_tx_id: u64) {
        self.graph
            .lock()
            .await
            .entry(tx_id)
            .or_insert_with(HashSet::new)
            .insert(dep_tx_id);
    }

    pub async fn detect_dealock(&self) -> Option<Vec<u64>> {
        let graph = { self.graph.lock().await.clone() };
        let mut recusion_set = HashSet::new();
        let mut visited = HashSet::new();
        for node in graph.keys() {
            let mut path = Vec::new();
            if let Some(path) = Self::dfs(*node, &graph, &mut visited, &mut recusion_set, &mut path)
            {
                return Some(path);
            }
        }

        None
    }

    fn dfs(
        node: u64,
        graph: &HashMap<u64, HashSet<u64>>,
        visited: &mut HashSet<u64>,
        recursion_set: &mut HashSet<u64>,
        path: &mut Vec<u64>,
    ) -> Option<Vec<u64>> {
        if !&visited.contains(&node) {
            visited.insert(node);
            recursion_set.insert(node);
            path.push(node);

            if let Some(nbs) = graph.get(&node) {
                for nb in nbs {
                    if !visited.contains(nb) {
                        if let Some(cycle) = Self::dfs(*nb, graph, visited, recursion_set, path) {
                            return Some(cycle);
                        }
                    } else if recursion_set.contains(nb) {
                        path.push(*nb);
                        return Some(Self::extract_cycle(path, *nb));
                    }
                }
            }
        }
        recursion_set.remove(&node);
        path.pop();
        None
    }

    fn extract_cycle(path: &[u64], start: u64) -> Vec<u64> {
        let pos = path.iter().position(|&x| x == start).unwrap_or(0);
        path[pos..].to_vec()
    }
}

impl Default for WaitGraph {
    fn default() -> Self {
        Self::new()
    }
}
