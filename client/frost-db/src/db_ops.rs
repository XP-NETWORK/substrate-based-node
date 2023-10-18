use rocksdb::{DBCompressionType, Options, DB};

pub struct FrostDb {
	db: DB,
}

impl FrostDb {
	/// Constructor: Open or create a RocksDB at the given path
	pub fn new(path_to_db: &str) -> Self {
		// 512 MB write buffer
		let write_buffer_size = 512 * 1024 * 1024;
		let mut db_options = Options::default();
		db_options.create_if_missing(true);
		db_options.increase_parallelism(4);
		// We use zstd compression methods as it provides a good balance between performance and compression ratio
		db_options.set_compression_type(DBCompressionType::Zstd);
		db_options.set_write_buffer_size(write_buffer_size);
		db_options.create_missing_column_families(true);
		let db = DB::open(&db_options, path_to_db).expect("Failed to open database Connection");
		FrostDb { db }
	}

	/// Insert or update key-value pair into the db instance
	pub fn upsert(&self, key: &[u8], value: &[u8]) {
		self.db.put(key, value).expect("Failed to insert value into the db");
	}

	/// Get the value responding to the given key from the DB
	pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
		self.db.get(key).ok().and_then(|opt| opt)
	}

	/// Remove the key-value pair with the given key
	pub fn remove(&self, key_to_remove: &[u8]) {
		self.db.delete(key_to_remove).expect("Failed to delete the entry");
	}
}
#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;
	use std::time::SystemTime;

	// Helper function to generate a unique path for each test
	fn get_unique_path() -> String {
		let timestamp: u128 =
			SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos();
		format!("test_db_{}", timestamp)
	}

	// Helper function to clean up the test DB directory after tests
	fn cleanup(path: &str) {
		let _ = fs::remove_dir_all(path);
	}

	#[test]
	fn test_frost_db_upsert_and_get() {
		let path = get_unique_path(); // This is a String
		let db = FrostDb::new(&path); // Convert String to &str here

		// Upsert a new key-value pair
		db.upsert(b"test_key", b"test_value");

		// Check the inserted value
		let retrieved_value = db.get(b"test_key").expect("Value should exist.");
		assert_eq!(retrieved_value, b"test_value");

		// Upsert (update) the same key with a new value
		db.upsert(b"test_key", b"new_value");

		// Check the updated value
		let retrieved_value = db.get(b"test_key").expect("Value should exist.");
		assert_eq!(retrieved_value, b"new_value");

		cleanup(&path); // Clean up after the test using &str
	}

	#[test]
	fn test_frost_db_remove() {
		let path = get_unique_path(); // This is a String
		let db = FrostDb::new(&path); // Convert String to &str here

		// Upsert a new key-value pair
		db.upsert(b"test_key", b"test_value");

		// Check the inserted value
		let retrieved_value = db.get(b"test_key").expect("Value should exist.");
		assert_eq!(retrieved_value, b"test_value");

		// Remove the key-value pair
		db.remove(b"test_key");

		// Check that the value doesn't exist anymore
		let retrieved_value = db.get(b"test_key");
		assert_eq!(retrieved_value, None);

		cleanup(&path); // Clean up after the test using &str
	}
}
