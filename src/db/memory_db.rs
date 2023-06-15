use std::collections::HashMap;

use uuid::Uuid;

use super::DB;

pub struct MemoryDB<T> {
    data: HashMap<Uuid, T>,
}

impl<T> Default for MemoryDB<T> {
    fn default() -> Self {
        Self { data: Default::default() }
    }
}

impl<T> DB<T> for MemoryDB<T> {
    fn put(&mut self, id: Uuid, item: T) {
        self.data.insert(id, item);
    }

    fn modify<F: FnOnce(&mut T)>(&mut self, id: Uuid, f: F) {
        self.data.entry(id).and_modify(f);
    }

    fn delete(&mut self, id: Uuid) {
        self.data.remove(&id);
    }

    fn fetch<F: Fn(&T)>(&mut self, f: F) {
        for v in self.data.values() {
            f(v);
        }
    }
}
