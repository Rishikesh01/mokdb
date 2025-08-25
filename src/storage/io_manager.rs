use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::errors::MokErrors;

use super::page::{Page, PAGE_SIZE};

pub struct IOManager {
    tables_parent_directory: PathBuf,
}

/*
* We will allow page
*
*/

impl IOManager {
    pub fn new(tables_parent_directory: PathBuf) -> Self {
        Self {
            tables_parent_directory,
        }
    }
    pub fn get_page(
        self,
        table_name: String,
        page_no: u64,
        buf: &mut [u8],
    ) -> Result<(), MokErrors> {
        todo!()
    }

    pub fn append_page(
        self,
        table_name: String,
        page_no: u64,
        buf: &[u8],
    ) -> Result<(), MokErrors> {
        todo!()
    }
    pub fn num_pages(&self, table: &str) -> Result<u64, MokErrors> {
        todo!()
    }
}
