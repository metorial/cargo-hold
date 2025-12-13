use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const EPOCH: u64 = 1640995200000;
const WORKER_ID_BITS: u64 = 5;
const DATACENTER_ID_BITS: u64 = 5;
const SEQUENCE_BITS: u64 = 12;

const MAX_WORKER_ID: u64 = (1 << WORKER_ID_BITS) - 1;
const MAX_DATACENTER_ID: u64 = (1 << DATACENTER_ID_BITS) - 1;
const MAX_SEQUENCE: u64 = (1 << SEQUENCE_BITS) - 1;

const WORKER_ID_SHIFT: u64 = SEQUENCE_BITS;
const DATACENTER_ID_SHIFT: u64 = SEQUENCE_BITS + WORKER_ID_BITS;
const TIMESTAMP_SHIFT: u64 = SEQUENCE_BITS + WORKER_ID_BITS + DATACENTER_ID_BITS;

pub struct SnowflakeGenerator {
    worker_id: u64,
    datacenter_id: u64,
    sequence: u64,
    last_timestamp: u64,
}

impl SnowflakeGenerator {
    pub fn new(worker_id: u64, datacenter_id: u64) -> Result<Self, String> {
        if worker_id > MAX_WORKER_ID {
            return Err(format!("Worker ID must be between 0 and {}", MAX_WORKER_ID));
        }
        if datacenter_id > MAX_DATACENTER_ID {
            return Err(format!(
                "Datacenter ID must be between 0 and {}",
                MAX_DATACENTER_ID
            ));
        }

        Ok(Self {
            worker_id,
            datacenter_id,
            sequence: 0,
            last_timestamp: 0,
        })
    }

    pub fn generate(&mut self) -> Result<i64, String> {
        let mut timestamp = self.current_timestamp()?;

        if timestamp < self.last_timestamp {
            return Err("Clock moved backwards".to_string());
        }

        if timestamp == self.last_timestamp {
            self.sequence = (self.sequence + 1) & MAX_SEQUENCE;
            if self.sequence == 0 {
                timestamp = self.wait_next_millis(self.last_timestamp)?;
            }
        } else {
            self.sequence = 0;
        }

        self.last_timestamp = timestamp;

        let id = ((timestamp - EPOCH) << TIMESTAMP_SHIFT)
            | (self.datacenter_id << DATACENTER_ID_SHIFT)
            | (self.worker_id << WORKER_ID_SHIFT)
            | self.sequence;

        Ok(id as i64)
    }

    fn current_timestamp(&self) -> Result<u64, String> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .map_err(|e| format!("Failed to get system time: {}", e))
    }

    fn wait_next_millis(&self, last_timestamp: u64) -> Result<u64, String> {
        let mut timestamp = self.current_timestamp()?;
        while timestamp <= last_timestamp {
            timestamp = self.current_timestamp()?;
        }
        Ok(timestamp)
    }
}

pub struct SnowflakeGeneratorWrapper(Mutex<SnowflakeGenerator>);

impl SnowflakeGeneratorWrapper {
    pub fn new(worker_id: u64, datacenter_id: u64) -> Result<Self, String> {
        Ok(Self(Mutex::new(SnowflakeGenerator::new(
            worker_id,
            datacenter_id,
        )?)))
    }

    pub fn generate(&self) -> Result<i64, String> {
        self.0
            .lock()
            .map_err(|e| format!("Lock poisoned: {}", e))?
            .generate()
    }
}

pub fn generate_prefixed_id(prefix: &str, oid: i64) -> String {
    use rand::Rng;
    let random_suffix: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(20)
        .map(char::from)
        .collect();
    format!("{}_{:x}{}", prefix, oid, random_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_snowflake_generator_creation() {
        let gen = SnowflakeGenerator::new(1, 1);
        assert!(gen.is_ok());
    }

    #[test]
    fn test_snowflake_generator_invalid_worker_id() {
        let gen = SnowflakeGenerator::new(32, 1);
        assert!(gen.is_err());
    }

    #[test]
    fn test_snowflake_generator_invalid_datacenter_id() {
        let gen = SnowflakeGenerator::new(1, 32);
        assert!(gen.is_err());
    }

    #[test]
    fn test_snowflake_generates_unique_ids() {
        let mut gen = SnowflakeGenerator::new(1, 1).unwrap();
        let mut ids = HashSet::new();

        for _ in 0..1000 {
            let id = gen.generate().unwrap();
            assert!(ids.insert(id), "Generated duplicate ID: {}", id);
        }
    }

    #[test]
    fn test_snowflake_generates_positive_ids() {
        let mut gen = SnowflakeGenerator::new(1, 1).unwrap();
        for _ in 0..100 {
            let id = gen.generate().unwrap();
            assert!(id > 0, "Generated non-positive ID: {}", id);
        }
    }

    #[test]
    fn test_snowflake_wrapper_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let gen = Arc::new(SnowflakeGeneratorWrapper::new(1, 1).unwrap());
        let mut handles = vec![];
        let mut all_ids = HashSet::new();

        for _ in 0..10 {
            let gen_clone = Arc::clone(&gen);
            let handle = thread::spawn(move || {
                let mut ids = vec![];
                for _ in 0..100 {
                    ids.push(gen_clone.generate().unwrap());
                }
                ids
            });
            handles.push(handle);
        }

        for handle in handles {
            let ids = handle.join().unwrap();
            for id in ids {
                assert!(all_ids.insert(id), "Generated duplicate ID: {}", id);
            }
        }

        assert_eq!(all_ids.len(), 1000);
    }

    #[test]
    fn test_generate_prefixed_id_format() {
        let id = generate_prefixed_id("test", 123);
        assert!(id.starts_with("test_"));
        assert!(id.contains("7b"));
        assert!(id.len() > 8);
    }

    #[test]
    fn test_generate_prefixed_id_uniqueness() {
        let mut ids = HashSet::new();
        for _ in 0..100 {
            let id = generate_prefixed_id("file", 1);
            ids.insert(id);
        }
        assert_eq!(ids.len(), 100);
    }
}
