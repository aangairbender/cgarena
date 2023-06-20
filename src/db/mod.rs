use uuid::Uuid;

pub mod json_db;
pub mod memory_db;

pub trait DB<T> {
    fn put(&self, id: Uuid, item: T);
    fn modify<F: FnOnce(&mut T)>(&self, id: Uuid, f: F);
    fn delete(&self, id: Uuid);
    fn fetch<F: Fn(&T)>(&self, f: F);
}
