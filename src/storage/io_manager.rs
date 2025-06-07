use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use super::page::{Page, PAGE_SIZE};

pub struct IOManager {
    tables_parent_directory: PathBuf,
}

impl IOManager {
    pub fn new(tables_parent_directory: PathBuf) -> Self {
        Self {
            tables_parent_directory,
        }
    }

    fn table_file_path(&self, table_name: &str) -> PathBuf {
        self.tables_parent_directory
            .join(format!("{}.tbl", table_name))
    }

    pub fn read_page(&self, table_name: &str, page_no: u64) -> io::Result<Page> {
        let file_path = self.table_file_path(table_name);
        let mut file = File::open(file_path)?;

        let offset = page_no * PAGE_SIZE as u64;
        file.seek(SeekFrom::Start(offset))?;

        let mut buffer = vec![0u8; PAGE_SIZE];
        file.read_exact(&mut buffer)?;

        Page::from_bytes(&buffer)
    }

    pub fn insert_page(&self, table_name: &str, page: &[u8; PAGE_SIZE]) -> io::Result<u64> {
        let file_path = self.table_file_path(table_name);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        file.write_all(page)?;

        let metadata = file.metadata()?;
        let file_len = metadata.len();
        let new_page_no = (file_len / PAGE_SIZE as u64) - 1;

        Ok(new_page_no)
    }

    pub fn flush_page(&self, table_name: &str, page: &Page) -> io::Result<()> {
        let file_path = self.table_file_path(table_name);
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(file_path)?;

        let offset = page.id * PAGE_SIZE as u64;
        file.seek(SeekFrom::Start(offset))?;

        let bytes = page.to_bytes();
        assert_eq!(
            bytes.len(),
            PAGE_SIZE,
            "Page must be exactly PAGE_SIZE bytes"
        );
        file.write_all(&bytes)?;

        Ok(())
    }
}
