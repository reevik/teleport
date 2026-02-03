use crate::paging::{Page, PAGE_SIZE};
use crate::types::o16;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

// in-memory cache which holds page ids to Page objects.
static CACHE: Lazy<Mutex<HashMap<o16, Arc<Page>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub(crate) fn write(page: &Page) {
    let page_id = page.page_id();
    let file_offset = page_id * PAGE_SIZE;
    let mut file = OpenOptions::new().write(true).open("index.000").unwrap();
    let _ = file.seek(SeekFrom::Start(file_offset.0 as u64));
    let _ = file.write_all(page.buffer());
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
    None
}
