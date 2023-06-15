use uuid::Uuid;

pub mod json_db;
pub mod memory_db;

pub trait DB<T> {
    fn put(&mut self, id: Uuid, item: T);
    fn modify<F: FnOnce(&mut T)>(&mut self, id: Uuid, f: F);
    fn delete(&mut self, id: Uuid);
    fn fetch<F: Fn(&T)>(&mut self, f: F);
}
