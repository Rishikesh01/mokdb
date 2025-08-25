use std::collections::HashSet;

use crate::engine::ir::{Column, TableLevelConstraints};

pub trait CatalogManager {
    fn get_table(&self, name: &str) -> Option<&Table>;
}

#[derive(Debug)]
pub struct Table {
    name: String,
    columns: Vec<Column>,
    set_columns: HashSet<String>,
    constraints: Option<Vec<TableLevelConstraints>>,
}

impl Table {
    pub fn new(
        name: String,
        columns: Vec<Column>,
        constraints: Option<Vec<TableLevelConstraints>>,
    ) -> Self {
        let set_columns = HashSet::from_iter(columns.iter().map(|x| x.name.clone()));
        Self {
            name,
            columns,
            set_columns,
            constraints,
        }
    }
    pub fn get_columns(&self) -> Vec<Column> {
        self.columns.clone()
    }
    pub fn contains_column(&self, name: &String) -> bool {
        self.set_columns.contains(name)
    }
}

pub struct CatalogManagerImpl {
    tables: Vec<Table>,
}

impl CatalogManagerImpl {
    pub fn new(tables: Vec<Table>) -> Self {
        Self { tables: tables }
    }
}

impl CatalogManager for CatalogManagerImpl {
    fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.name == name)
    }
}
