use crate::paging::{Page, PAGE_SIZE, PAGE_SIZE_USIZE};
use crate::types::o16;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

// in-memory cache which holds page ids to Page objects.
static CACHE: Lazy<Mutex<HashMap<o16, Arc<Page>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn write(page: Page) {
    let page_id = page.page_id();
    let file_offset = page_id * PAGE_SIZE;
    let mut file = OpenOptions::new().write(true).create(true).open("index.000").unwrap();
    let _ = file.seek(SeekFrom::Start(file_offset.0 as u64));
    let _ = file.write_all(page.buffer());
    let mut cache = CACHE.lock().unwrap();
    cache.insert(page.page_id(), Arc::new(page));
}

pub(crate) fn read(page_id: usize) -> Option<Arc<Page>> {
    let id = o16(page_id as u16);
    let cache = CACHE.lock().unwrap();
    let page = match cache.get(&id).cloned() {
        None => read_from_disk(page_id),
        Some(found) => Some(found),
    };
    page
}

fn read_from_disk(page_id: usize) -> Option<Arc<Page>> {
    let file_offset = page_id * PAGE_SIZE_USIZE;
    let mut file = OpenOptions::new().write(true).create(true).open("index.000").unwrap();
    file.seek(SeekFrom::Start(file_offset as u64)).unwrap();
    let mut buffer = [0u8; PAGE_SIZE_USIZE];
    file.read_exact(&mut buffer).unwrap();
    let new_page = Page::new_from(buffer);
    Some(Arc::new(new_page))
}

pub(crate) fn delete_index() {
    match fs::remove_file("index.000") {
        Ok(_) => println!("index.000 deleted."),
        Err(_) => println!("index.000 not found."),
    }
}