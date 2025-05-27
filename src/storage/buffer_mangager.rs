use std::{
    collections::{hash_map::Entry, HashMap},
    sync::atomic::AtomicBool,
};

use super::{io_manager::IOManager, page::Page};

struct BufferFrame {
    spin_lock: AtomicBool,
    page: Page,
    pin_count: u32,
    is_dirty: bool,
}

struct TablePages {
    pages: HashMap<u64, BufferFrame>,
}

pub struct BufferManager {
    io_manager: IOManager,
    tables: HashMap<String, TablePages>,
}

impl BufferManager {
    pub fn new(io_manager: IOManager) -> Self {
        Self {
            tables: HashMap::new(),
            io_manager,
        }
    }

    pub async fn get_page_for_read(&mut self, table: &str, page_no: u64) -> Option<&Page> {
        let table_pages = self
            .tables
            .entry(table.to_string())
            .or_insert_with(|| TablePages {
                pages: HashMap::new(),
            });
        if let Entry::Vacant(e) = table_pages.pages.entry(page_no) {
            match self.io_manager.read_page(table, page_no) {
                Ok(p) => {
                    e.insert(BufferFrame {
                        page: p,
                        pin_count: 0,
                        is_dirty: false,
                        spin_lock: AtomicBool::new(false),
                    });
                }
                Err(e) => {
                    eprintln!("failed to read page from disk: {}", e);
                    return None;
                }
            }
        }
        table_pages.pages.get_mut(&page_no).map(|frame| {
            frame.pin_count += 1;
            &frame.page
        })
    }

    pub fn get_page_for_write(&mut self, table: &str, page_no: u64) -> Option<&mut Page> {
        let table_pages = self
            .tables
            .entry(table.to_string())
            .or_insert_with(|| TablePages {
                pages: HashMap::new(),
            });
        if let Entry::Vacant(e) = table_pages.pages.entry(page_no) {
            match self.io_manager.read_page(table, page_no) {
                Ok(p) => {
                    e.insert(BufferFrame {
                        page: p,
                        pin_count: 0,
                        is_dirty: false,
                        spin_lock: AtomicBool::new(false),
                    });
                }
                Err(e) => {
                    eprintln!("failed to read page from disk: {}", e);
                    return None;
                }
            }
        }
        table_pages.pages.get_mut(&page_no).map(|frame| {
            while frame
                .spin_lock
                .swap(true, std::sync::atomic::Ordering::Acquire)
            {
                std::hint::spin_loop();
            }
            frame.pin_count += 1;
            &mut frame.page
        })
    }

    pub fn unpin_page(&mut self, table_name: &str, page_id: u64) {
        if let Some(frame) = self
            .tables
            .get_mut(table_name)
            .unwrap_or_else(|| panic!("the table should have been present {}", table_name))
            .pages
            .get_mut(&page_id)
        {
            if frame.pin_count > 0 {
                frame.pin_count -= 1;
                frame
                    .spin_lock
                    .store(false, std::sync::atomic::Ordering::Release);
            }
        }
    }

    pub fn flush_dirty_pages(&mut self) {
        for table in self.tables.iter_mut() {
            for frame in &mut table.1.pages.iter_mut() {
                if frame.1.is_dirty {
                    if let Err(e) = self.io_manager.update_page(table.0, &frame.1.page) {
                        eprintln!("error occured while flushing dirty pages: {}", e)
                    }
                    frame.1.is_dirty = false;
                }
            }
        }
    }

    pub fn mark_dirty(&mut self, table: &str, page_no: u64) {
        if let Some(frame) = self
            .tables
            .get_mut(table)
            .and_then(|tp| tp.pages.get_mut(&page_no))
        {
            frame.is_dirty = true;
        }
    }
}
