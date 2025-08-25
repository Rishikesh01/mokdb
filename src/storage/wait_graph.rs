/*
* We use it for deadlock
* Idea is to have nodes represent transaction
* Edge represent direction of waiting i.e Node A waiting on Node B which implies Transction A is
* waiting for Transaction B to release a resouce
*
* We use wait graph to find cyclic graphs i.e when you tranverse from Node A and you reach Node A
* again by following all the edges or subedges
*
*/

use std::collections::{HashMap, HashSet};

use dashmap::{DashMap, DashSet};

pub struct WaitGraph {
    nodes: DashMap<u64, DashSet<u64>>,
}

impl WaitGraph {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
        }
    }

    pub fn add_edge(&self, from: u64, to: u64) {
        self.nodes.entry(from).or_default().insert(to);
        self.nodes.entry(to).or_default();
    }

    pub fn remove_node(&self, tx_id: u64) {
        self.nodes.remove(&tx_id);
        for entry in self.nodes.iter_mut() {
            entry.value().remove(&tx_id);
        }
    }

    fn snapshot(&self) -> HashMap<u64, HashSet<u64>> {
        let mut snapshot_map = HashMap::new();
        for entry in self.nodes.iter() {
            snapshot_map.insert(*entry.key(), entry.value().iter().map(|v| *v).collect());
        }
        snapshot_map
    }

    pub fn detect_deadlock(&self) -> Option<Vec<u64>> {
        let graph = self.snapshot();

        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut on_stack = HashSet::new();

        for &node in graph.keys() {
            if !visited.contains(&node) {
                if let Some(cycle) =
                    self.dfs_find_cycle(node, &graph, &mut visited, &mut stack, &mut on_stack)
                {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /*
     * Basically what this does:
     * You have Node A with edges to B, C, D.
     *
     * DFS starts at Node A and explores each edge one by one:
     *   - Follow A → B, go as deep as possible from B
     *   - Backtrack, then follow A → C, explore fully
     *   - Backtrack, then follow A → D, explore fully
     *
     * While exploring, we keep a "stack" of the current DFS path.
     * If we encounter a node that is already on this stack, it means
     * we found a cycle. The stack then contains the full path of the
     * cycle, which we can return.
     *
     * So essentially:
     *   1. Iterate over each edge of the current node
     *   2. Recursively explore each edge
     *   3. Detect cycles using the DFS path via on stack
     *   4. Return the cycle path when found
     */

    fn dfs_find_cycle(
        &self,
        node: u64,
        graph: &HashMap<u64, HashSet<u64>>,
        visited: &mut HashSet<u64>,
        stack: &mut Vec<u64>,
        on_stack: &mut HashSet<u64>,
    ) -> Option<Vec<u64>> {
        visited.insert(node);
        on_stack.insert(node);
        stack.push(node);

        if let Some(neighbors) = graph.get(&node) {
            for &next in neighbors {
                if on_stack.contains(&next) {
                    let idx = stack.iter().position(|&x| x == next).unwrap();
                    let mut cycle = stack[idx..].to_vec();
                    cycle.push(next);
                    return Some(cycle);
                }

                if !visited.contains(&next) {
                    if let Some(cycle) = self.dfs_find_cycle(next, graph, visited, stack, on_stack)
                    {
                        return Some(cycle);
                    }
                }
            }
        }

        stack.pop();
        on_stack.remove(&node);

        None
    }
}
