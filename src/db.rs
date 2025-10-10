
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use bincode::{Decode, Encode};
use std::collections::HashMap;
use std::path::Path;
use rayon::prelude::*;

const LOCATIONS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("locations");

#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct FileMetadata {
    pub normalized_name: String,
    pub modified_time: u64,
}

pub struct DbManager {
    db: Database,
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

    pub fn read_index_for_path(&self, root_path: &str) -> anyhow::Result<HashMap<String, FileMetadata>> {
        let table_name_opt = self.get_table_name(root_path)?;
        if table_name_opt.is_none() {
            return Ok(HashMap::new());
        }
        let table_name = table_name_opt.unwrap();
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&table_name);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def)?;

        let mut map = HashMap::new();
        for item_result in table.iter()? {
            if let Ok(item) = item_result {
                let key = item.0.value().to_string();
                let value_bytes = item.1.value();
                let (value, _len): (FileMetadata, usize) = bincode::decode_from_slice(value_bytes, bincode::config::standard())?;
                map.insert(key, value);
            }
        }
        Ok(map)
    }

    pub fn write_index_for_path(&self, root_path: &str, files: &[(String, FileMetadata)]) -> anyhow::Result<()> {
        let table_name = self.get_or_create_table_name(root_path)?;
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&table_name);

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

    pub fn update_index_for_path(
        &self,
        root_path: &str,
        updates: &[(String, FileMetadata)],
        deletes: &[String],
    ) -> anyhow::Result<()> {
        let table_name = self.get_or_create_table_name(root_path)?;
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(&table_name);

        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(table_def)?;
            for path in deletes {
                table.remove(path.as_str())?;
            }
            for (path, metadata) in updates {
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

    pub fn search_in_table(&self, table_name: &str, query: &str) -> anyhow::Result<Vec<String>> {
        let table_def: TableDefinition<&str, &[u8]> = TableDefinition::new(table_name);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(table_def)?;

        let entries: Vec<_> = table.iter()?.filter_map(Result::ok).collect();

        let results = entries
            .par_iter()
            .filter_map(|(key, value)| {
                let value_bytes = value.value();
                if let Ok((metadata, _len)) = bincode::decode_from_slice::<FileMetadata, _>(value_bytes, bincode::config::standard()) {
                    if metadata.normalized_name.contains(query) {
                        return Some(key.value().to_string());
                    }
                }
                None
            })
            .collect();

        Ok(results)
    }

    fn get_table_name(&self, root_path: &str) -> anyhow::Result<Option<String>> {
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

    fn get_or_create_table_name(&self, root_path: &str) -> anyhow::Result<String> {
        if let Some(name) = self.get_table_name(root_path)? {
            return Ok(name);
        }
        self.create_table_name(root_path)
    }
}
