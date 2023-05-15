use std::{fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};
use fs2::FileExt;

pub struct JsonDB<T> {
    db_file: PathBuf,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + for<'a> Deserialize<'a>> JsonDB<T> {
    pub fn new(db_file: PathBuf) -> Self {
        Self {
            db_file,
            _phantom: Default::default(),
        }
    }

    pub fn load(&self) -> Result<Vec<T>, io::Error> {
        let json = fs::read_to_string(&self.db_file)?;
        let data = serde_json::from_str(&json)?;
        Ok(data)
    }

    pub fn save(&self, data: Vec<T>) -> Result<(), io::Error> {
        let json = serde_json::to_string(&data)?;
        fs::write(&self.db_file, json)?;
        Ok(())
    }

    pub fn modify<F>(&self, f: F) -> Result<(), io::Error>
    where
        F: FnOnce(&mut Vec<T>),
    {
        let file = fs::File::open(&self.db_file)?;
        file.lock_exclusive()?;
        let mut data = self.load()?;
        f(&mut data);
        self.save(data)?;
        file.unlock()?;
        Ok(())
    }
}
