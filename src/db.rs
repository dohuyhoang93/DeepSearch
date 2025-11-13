use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};
use std::path::Path;
use rayon::prelude::*;
use crate::utils;

const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");

#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct FileMetadata {
    pub normalized_name: String,
    pub modified_time: u64,
}

pub struct DbManager {
    pub db: Database,
}

impl DbManager {
    pub fn new(db_path: &Path) -> anyhow::Result<Self> {
        let db = Database::create(db_path)?;
        let txn = db.begin_write()?;
        {
            txn.open_table(LOCATIONS_TABLE)?;
        }
        txn.commit()?;
        Ok(Self { db })
    }



    pub fn write_to_table(&self, table_name: &str, files: &[(String, FileMetadata)]) -> anyhow::Result<()> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(table_name);

        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def)?;
            for (path, metadata) in files {
                let key = path.as_str();
                let value = bincode::encode_to_vec(metadata, bincode::config::standard())?;
                table.insert(key, &value[..])?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    pub fn get_all_locations(&self) -> anyhow::Result<Vec<(String, String)>> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(LOCATIONS_TABLE)?;
        Ok(table.iter()?.filter_map(Result::ok).map(|(path, table_name)| (path.value().to_string(), table_name.value().to_string())).collect())
    }

    pub fn get_table_len(&self, table_name: &str) -> anyhow::Result<u64> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(table_name);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def)?;
        Ok(table.len()?)
    }

    pub fn delete_location(&self, path_to_delete: &str) -> anyhow::Result<()> {
        let txn = self.db.begin_write()?;
        {
            let mut locations_table = txn.open_table(LOCATIONS_TABLE)?;

            // Get the table name, and then drop the immutable borrow.
            let table_name_to_delete: Option<String> = locations_table
                .get(path_to_delete)?
                .map(|guard| guard.value().to_string());

            if let Some(table_name) = table_name_to_delete {
                let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&table_name);
                
                // Now we can safely get mutable borrows
                txn.delete_table(table_def)?;
                locations_table.remove(path_to_delete)?;
            }
        }
        txn.commit()?;
        Ok(())
    }

    pub fn search_in_table(&self, table_name: &str, query: &str) -> anyhow::Result<Vec<String>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(table_name);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def)?;

        // Token-based search: split the query into words
        let query_tokens: Vec<&str> = query.split_whitespace().collect();
        if query_tokens.is_empty() {
            return Ok(Vec::new());
        }

        let results = table.iter()?
            .par_bridge()
            .filter_map(|item_result| {
                let (key, value) = item_result.ok()?;

                let value_bytes = value.value();
                if let Ok((metadata, _len)) = bincode::decode_from_slice::<FileMetadata, _>(value_bytes, bincode::config::standard()) {
                    // Check if all tokens are present in the normalized name
                    if utils::contains_all_tokens(&metadata.normalized_name, &query_tokens) {
                        return Some(key.value().to_string());
                    }
                }
                None
            })
            .collect();

        Ok(results)
    }

    pub fn get_table_name(&self, root_path: &str) -> anyhow::Result<Option<String>> {
        let txn = self.db.begin_read()?;
        let locations_table = txn.open_table(LOCATIONS_TABLE)?;
        Ok(locations_table.get(root_path)?.map(|name| name.value().to_string()))
    }
    
    fn create_table_name(&self, root_path: &str) -> anyhow::Result<String> {
        let write_txn = self.db.begin_write()?;
        let final_table_name;
        {
            let mut locations_table = write_txn.open_table(LOCATIONS_TABLE)?;
            
            let maybe_name = {
                // Limit the scope of the borrow from .get() to this block
                locations_table.get(root_path)?.map(|guard| guard.value().to_string())
            };

            if let Some(name) = maybe_name {
                final_table_name = name;
            } else {
                let new_name = format!("index_{:x}", md5::compute(root_path.as_bytes()));
                locations_table.insert(root_path, new_name.as_str())?;
                final_table_name = new_name;
            }
        }
        write_txn.commit()?;
        Ok(final_table_name)
    }

    pub fn get_or_create_table_name(&self, root_path: &str) -> anyhow::Result<String> {
        if let Some(name) = self.get_table_name(root_path)? {
            return Ok(name);
        }
        self.create_table_name(root_path)
    }

    pub fn swap_location_table(&self, root_path: &str, new_table_name: &str) -> anyhow::Result<String> {
        let txn = self.db.begin_write()?;
        let old_table_name;
        {
            let mut locations_table = txn.open_table(LOCATIONS_TABLE)?;
            // Get the old table name
            old_table_name = locations_table.get(root_path)?
                .map(|guard| guard.value().to_string())
                .ok_or_else(|| anyhow::anyhow!("Could not find old table name for location '{}'", root_path))?;

            // Point the location to the new table
            locations_table.insert(root_path, new_table_name)?;
        }
        txn.commit()?;
        Ok(old_table_name)
    }
}
