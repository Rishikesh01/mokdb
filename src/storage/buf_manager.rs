use std::{collections::VecDeque, sync::Arc};

use crate::storage::{io_manager::IOManager, page::Page};
use dashmap::{DashMap, DashSet};
use parking_lot::Mutex;

struct BufferFrame {
    spin_lock: Mutex<bool>,
    page: Page,
    pin_count: u32,
    is_dirty: bool,
}
pub struct BufferManager {
    io_manager: IOManager,
    load_queue: DashSet<(String, u64)>,
    pages: DashMap<(String, u64), Arc<BufferFrame>>,
    order: Mutex<VecDeque<Page>>,
}

impl BufferManager {
    pub fn new(io_manager: IOManager) -> Self {
        Self {
            io_manager,
            pages: DashMap::new(),
            order: Mutex::new(VecDeque::new()),
            load_queue: DashSet::new(),
        }
    }
}
