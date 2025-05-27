use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColumnDataType {
    Int,
    String,
    Float,
}

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: ColumnDataType,
}

#[derive(Debug)]
pub struct TableMetadata {
    pub columns: Vec<Column>,        // maintains order
    pub column_set: HashSet<String>, // for quick lookup
}

impl TableMetadata {
    pub fn new(columns: Vec<Column>) -> Self {
        let column_set = columns.iter().map(|col| col.name.clone()).collect();
        Self {
            columns,
            column_set,
        }
    }

    pub fn remove_column(&mut self, column_name: &str) {
        self.columns.retain(|col| col.name != column_name);
        self.column_set.remove(column_name);
    }

    pub fn add_column(&mut self, column: Column) {
        if self.column_set.insert(column.name.clone()) {
            self.columns.push(column);
        }
    }
}

#[derive(Debug, Default)]
pub struct CatalogManager {
    pub tables: HashMap<String, TableMetadata>,
}

impl CatalogManager {
    pub fn new(reserved_tables: Vec<(String, Vec<Column>)>) -> Self {
        let tables = reserved_tables
            .into_iter()
            .map(|(table_name, columns)| (table_name, TableMetadata::new(columns)))
            .collect();

        Self { tables }
    }

    pub fn create_table(&mut self, table_name: String, columns: Vec<Column>) -> Option<()> {
        if self.tables.contains_key(&table_name) {
            return None;
        }
        self.tables.insert(table_name, TableMetadata::new(columns));
        Some(())
    }

    pub fn get_table_metadata(&self, table_name: &str) -> Option<&TableMetadata> {
        self.tables.get(table_name)
    }

    pub fn remove_column_from_table(&mut self, table_name: &str, column_name: &str) -> Option<()> {
        self.tables.get_mut(table_name)?.remove_column(column_name);
        Some(())
    }

    pub fn add_column_in_table(&mut self, table_name: &str, column: Column) -> Option<()> {
        self.tables.get_mut(table_name)?.add_column(column);
        Some(())
    }
}
