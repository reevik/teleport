use crate::errors::InvalidPageOffsetError;
use crate::types::PayloadType::Str;
use crate::types::{o16, FromLeBytes, Key, Payload, PayloadType, ToLeBytes};
use alloc::vec::Vec;
use once_cell::sync::Lazy;
use rand::Rng;
use std::cmp::min;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::Read;
use std::sync::Mutex;

static CACHE: Lazy<Mutex<HashMap<o16, Page>>> = Lazy::new(|| Mutex::new(HashMap::new()));

const ZERO: o16 = o16(0);
static mut NEXT_PAGE_ID: o16 = o16(0);
const PAGE_SIZE: o16 = o16(4096);
const PAGE_SIZE_USIZE: usize = PAGE_SIZE.0 as usize;

const SIZE_NUM_OF_SLOTS: usize = size_of::<o16>();
const SIZE_PAGE_ID: usize = size_of::<o16>();
const SIZE_PAGE_TYPE: usize = size_of::<u8>();
const SIZE_FLAGS: usize = size_of::<u8>();
const SIZE_LEFT_MOST: usize = size_of::<o16>();
const SIZE_LEFT_SIBLING: usize = size_of::<o16>();
const SIZE_RIGHT_SIBLING: usize = size_of::<o16>();
const SIZE_PARENT_PAGE_ID: usize = size_of::<o16>();
const SIZE_FREE_START: usize = size_of::<o16>();
const SIZE_FREE_END: usize = size_of::<o16>();
const SIZE_OF_SLOT_TABLE_ITEM: usize = size_of::<o16>();
const SIZE_OFFSET: usize = size_of::<o16>();

pub const TOTAL_HEADER_SIZE: usize = SIZE_FLAGS
    + SIZE_RIGHT_SIBLING
    + SIZE_LEFT_SIBLING
    + SIZE_LEFT_MOST
    + SIZE_PARENT_PAGE_ID
    + SIZE_PAGE_ID
    + SIZE_PAGE_TYPE
    + SIZE_NUM_OF_SLOTS
    + SIZE_FREE_START
    + SIZE_FREE_END;

/// Offsets in the
const OFFSET_NUM_OF_SLOTS: usize = 0;
const OFFSET_PAGE_ID: usize = OFFSET_NUM_OF_SLOTS + SIZE_NUM_OF_SLOTS;
const OFFSET_PAGE_TYPE: usize = OFFSET_PAGE_ID + SIZE_PAGE_ID;
const OFFSET_FLAGS: usize = OFFSET_PAGE_TYPE + SIZE_PAGE_TYPE;
const OFFSET_LEFT_MOST: usize = OFFSET_FLAGS + SIZE_FLAGS;
const OFFSET_LEFT_SIBLING: usize = OFFSET_LEFT_MOST + SIZE_LEFT_MOST;
const OFFSET_RIGHT_SIBLING: usize = OFFSET_LEFT_SIBLING + SIZE_LEFT_SIBLING;
const OFFSET_PARENT_PAGE_ID: usize = OFFSET_RIGHT_SIBLING + SIZE_RIGHT_SIBLING;
const OFFSET_FREE_START: usize = OFFSET_PARENT_PAGE_ID + SIZE_PARENT_PAGE_ID;
const OFFSET_FREE_END: usize = OFFSET_FREE_START + SIZE_FREE_START;

#[derive(Clone, Copy)]
pub struct Page {
    buffer: [u8; PAGE_SIZE_USIZE],
}

enum PageType {
    INNER,
    LEAF,
}

impl From<u8> for PageType {
    fn from(value: u8) -> Self {
        if value == 0 {
            return Self::INNER;
        }
        Self::LEAF
    }
}

const DATA_PAGE: u8 = 0;
const INNER_PAGE: u8 = 1;

impl Page {
    fn new(page_type: u8) -> Self {
        let mut new_instance = Self {
            buffer: [0u8; PAGE_SIZE_USIZE],
        };

        new_instance.set_flags(0);
        new_instance.set_left_most_page_id(ZERO);
        new_instance.set_right_sibling(ZERO);
        new_instance.set_left_sibling(ZERO);
        new_instance.set_parent(ZERO);
        new_instance.set_num_of_slots(ZERO);
        new_instance.set_free_start(TOTAL_HEADER_SIZE.try_into().expect("Too many pages"));
        new_instance.set_free_end(PAGE_SIZE.try_into().expect(""));
        new_instance.set_page_type(page_type);
        unsafe {
            new_instance.set_page_id(NEXT_PAGE_ID);
            NEXT_PAGE_ID = o16(NEXT_PAGE_ID.0 + 1);
        }
        new_instance
    }

    pub fn new_leaf(key: Key, payload: Payload) -> Result<o16, InvalidPageOffsetError> {
        let mut cache = CACHE.lock().unwrap();
        let parent_page = Self::new(DATA_PAGE);
        let current_page_id = parent_page.page_id();
        cache.insert(current_page_id, parent_page);
        let current_page = cache.get_mut(&current_page_id).expect("");
        let mut residual = current_page.add_key_data(key, payload).expect("");
        let mut prev_page_id = current_page_id;
        while residual.len() > 0 {
            let mut overflow_page = Self::new(DATA_PAGE);
            let overflow_page_id = overflow_page.page_id();
            residual = overflow_page.add_overflow_data(residual).expect("");
            cache.insert(overflow_page_id, overflow_page);
            let prev_page = cache.get_mut(&prev_page_id).expect("");
            prev_page.set_right_sibling(overflow_page_id);
            prev_page_id = overflow_page_id;
        }
        Ok(current_page_id)
    }

    pub fn new_inner() -> Self {
        Self::new(INNER_PAGE)
    }

    pub fn add_left_most(&mut self, left_most_page_id: o16) {
        self.set_left_most_page_id(left_most_page_id);
    }

    fn add_key_ref(&mut self, key: Key, payload: Payload) -> Result<(), InvalidPageOffsetError> {
        match self.add_key_data(key, payload) {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    // Adds data into a leaf node.
    fn add_key_data(
        &mut self,
        key: Key,
        mut payload: Payload,
    ) -> Result<Payload, InvalidPageOffsetError> {
        // determine the payload and key size.
        let payload_ref = &payload;
        let key_buf = key.to_bytes();
        let key_buf_size = key_buf.len();
        let payload_size = payload.len();
        let payload_type = payload_ref.payload_type;
        let key_type: PayloadType = Str;

        // each for payload and key size plus the slot table entry (offset) hence three.
        let header_size = 3 * SIZE_OFFSET + 2 * SIZE_FLAGS;
        let free_space: usize = self.free_size().try_into()?;
        let available_net_free_space = free_space - header_size - key_buf_size;
        // consume the payload for available net space or payload size if it is smaller than available net space.
        let mut read_buf = vec![0; min(available_net_free_space, payload_size)];
        let _ = payload.read(&mut read_buf);
        let mut slot: Vec<u8> =
            Vec::with_capacity(Self::slot_size(key_buf_size, payload_size).try_into()?);
        let payload_size_in_o16: o16 = payload_size.try_into()?;
        let key_buf_size_in_o16: o16 = key_buf_size.try_into()?;
        slot.extend_from_slice(&payload_size_in_o16.to_bytes());
        slot.extend_from_slice(&[payload_type as u8]);
        slot.extend_from_slice(&key_buf_size_in_o16.to_bytes());
        slot.extend_from_slice(&[key_type as u8]);
        slot.extend_from_slice(key_buf.as_slice());
        slot.extend_from_slice(&read_buf);
        let new_free_end = self.add_slot(&mut slot)?;
        // advance the free start and slot table with the new free end.
        self.add_to_slot_table(new_free_end)?;
        Ok(payload)
    }

    // Read offset payload as a vector of bytes.
    fn get_overflow_data(&self) -> Result<Vec<u8>, InvalidPageOffsetError> {
        let offset_index = TOTAL_HEADER_SIZE;
        let slot_offset = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            offset_index,
            o16::from_bytes,
        );

        let payload_len = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            slot_offset.try_into()?,
            o16::from_bytes,
        );

        let slot_offset_usize: usize = slot_offset.try_into()?;
        let payload_offset = slot_offset_usize + SIZE_PAGE_ID;
        let payload: Vec<u8> = Self::read_le_into_buffer::<Vec<u8>>(
            &self.buffer,
            payload_offset,
            payload_len.try_into()?,
            |b| b,
        );

        Ok(payload)
    }
    fn add_overflow_data(
        &mut self,
        mut payload: Payload,
    ) -> Result<Payload, InvalidPageOffsetError> {
        let max_available_payload_size: usize = self
            .max_available_payload_size_in_overflow_page()
            .try_into()
            .expect("Too many pages");
        let copy_size = min(payload.len(), max_available_payload_size);
        let mut payload_in_bytes: Vec<u8> = vec![0; copy_size];
        let _ = payload.read(&mut payload_in_bytes);
        let copy_size_o16: o16 = copy_size.try_into().expect("Too many pages");
        let mut slot: Vec<u8> = Vec::with_capacity(copy_size);
        slot.extend_from_slice(&copy_size_o16.to_bytes());
        slot.extend_from_slice(&payload_in_bytes);
        let new_free_end = self.add_slot(&mut slot)?;
        // advance the free start and slot table with the new free end.
        self.add_to_slot_table(new_free_end)?;
        Ok(payload)
    }

    fn max_available_payload_size_in_overflow_page(&self) -> o16 {
        self.free_size() - 2 * SIZE_OF_SLOT_TABLE_ITEM
    }

    fn slot_size(key_len: usize, payload_len: usize) -> o16 {
        (2 * size_of::<o16>() + 2 * size_of::<u8>() + key_len + payload_len)
            .try_into()
            .expect("Too many pages")
    }

    // new_free_end is the new position of the free_end after inserting a new slot at the end of the
    // page. The slots make the page grow backward:
    // | Page Header | slot table | ... free space ... | new slot | prev slot | .. |
    fn add_to_slot_table(&mut self, new_free_end: o16) -> Result<(), InvalidPageOffsetError> {
        let free_start = self.free_start();
        let new_free_end_offset = &new_free_end.to_bytes();
        let start: usize = free_start.try_into()?;
        let end: usize = start + SIZE_OF_SLOT_TABLE_ITEM;
        self.buffer[start..end].copy_from_slice(new_free_end_offset);
        let size_of_slot_table_item: o16 = SIZE_OF_SLOT_TABLE_ITEM.try_into()?;
        self.set_free_start(free_start + size_of_slot_table_item);
        self.set_num_of_slots(self.num_of_slots() + 1);
        debug_assert!(self.free_start() <= self.free_end());
        Ok(())
    }

    fn get_key_payload(&self, index: o16) -> Result<String, InvalidPageOffsetError> {
        let index_usize: usize = index.try_into()?;
        let o16_size: usize = size_of::<o16>();
        let offset_index = TOTAL_HEADER_SIZE + (index_usize * o16_size);

        let slot_offset = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            offset_index,
            o16::from_bytes,
        );

        let payload_len = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            slot_offset.try_into()?,
            o16::from_bytes,
        );

        let slot_offset_usize: usize = slot_offset.try_into()?;
        let page_type_size: usize = SIZE_PAGE_ID;
        let flag_offset = slot_offset_usize + page_type_size;
        let payload_type =
            Self::read_le::<u8, SIZE_FLAGS>(&self.buffer, flag_offset, u8::from_bytes);

        let key_len_offset = flag_offset + SIZE_FLAGS;
        let key_len = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            key_len_offset,
            o16::from_bytes,
        );

        let key_type_offset = key_len_offset + SIZE_PAGE_ID;
        let key_offset = key_type_offset + SIZE_FLAGS;
        let key_len_usize: usize = key_len.try_into()?;

        let page_size: usize = PAGE_SIZE.try_into()?;
        let max_payload_capacity =
            page_size - (key_len_usize + TOTAL_HEADER_SIZE + 3 * SIZE_PAGE_ID + 2 * SIZE_PAGE_TYPE);
        let stringify = |b: Vec<u8>| String::from_utf8_lossy(b.as_slice()).to_string();
        let payload_offset = key_offset + key_len_usize;
        let mut payload = Self::read_le_into_buffer::<Vec<u8>>(
            &self.buffer,
            payload_offset,
            min(max_payload_capacity, payload_len.try_into()?),
            |b| b,
        );

        let mut current_right_sibling = self.right_sibling();
        if current_right_sibling == o16(0) {
            return Ok(stringify(payload));
        }

        loop {
            if current_right_sibling == o16(0) {
                break;
            }
            current_right_sibling = match CACHE.lock().unwrap().get(&current_right_sibling) {
                Some(overflow_page) => {
                    if let Ok(overflow_data) = overflow_page.get_overflow_data() {
                        payload.extend_from_slice(&overflow_data);
                    }
                    overflow_page.right_sibling()
                }
                None => {
                    panic!("Page ID out of bounds");
                }
            };
        }

        Ok(stringify(payload))
    }

    fn add_slot(&mut self, slot: &Vec<u8>) -> Result<o16, InvalidPageOffsetError> {
        let free_end = self.free_end();
        let new_free_end = free_end - slot.len();
        // update the buffer with key-payload.
        self.buffer[new_free_end.try_into().expect("")..free_end.try_into()?]
            .copy_from_slice(&slot);
        self.set_free_end(new_free_end);
        debug_assert!(self.free_start() <= self.free_end());
        // As we reverse traverse the slot blocks, the old free_end becomes the start of the slot.
        Ok(new_free_end)
    }

    pub(crate) fn free_size(&self) -> o16 {
        self.free_end() - self.free_start()
    }

    fn read_le<T, const N: usize>(buf: &[u8], offset: usize, f: fn(Vec<u8>) -> T) -> T {
        let slice = &buf[offset..offset + N];
        let arr: [u8; N] = slice.try_into().expect("slice length mismatch");
        f(arr.to_vec())
    }

    fn read_le_into_buffer<T>(buf: &[u8], offset: usize, length: usize, f: fn(Vec<u8>) -> T) -> T {
        let buffer_ref = buf[offset..offset + length].to_vec();
        f(buffer_ref)
    }

    fn write_le<T, const N: usize>(buf: &mut [u8], offset: usize, value: T, f: fn(T) -> Vec<u8>) {
        let bytes = f(value);
        buf[offset..offset + N].copy_from_slice(&bytes);
    }

    /// Returns the number of slots from the first two bytes in the page.
    fn num_of_slots(&self) -> o16 {
        Self::read_le::<o16, SIZE_NUM_OF_SLOTS>(&self.buffer, OFFSET_NUM_OF_SLOTS, o16::from_bytes)
    }

    fn set_num_of_slots(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_NUM_OF_SLOTS>(
            &mut self.buffer,
            OFFSET_NUM_OF_SLOTS,
            num,
            |value| value.to_bytes(),
        );
    }

    fn page_id(&self) -> o16 {
        Self::read_le::<o16, SIZE_PAGE_ID>(&self.buffer, OFFSET_PAGE_ID, |v| o16::from_bytes(v))
    }

    fn set_page_id(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_PAGE_ID>(&mut self.buffer, OFFSET_PAGE_ID, num, |value| {
            value.to_bytes()
        });
    }

    fn page_type(&self) -> u8 {
        Self::read_le::<u8, SIZE_PAGE_TYPE>(&self.buffer, OFFSET_PAGE_TYPE, |value| {
            u8::from_bytes(value)
        })
    }

    fn set_page_type(&mut self, num: u8) {
        Self::write_le::<u8, SIZE_PAGE_TYPE>(&mut self.buffer, OFFSET_PAGE_TYPE, num, |value| {
            value.to_le_bytes().to_vec()
        });
    }

    fn flags(&self) -> u8 {
        Self::read_le::<u8, SIZE_FLAGS>(&self.buffer, OFFSET_FLAGS, u8::from_bytes)
    }

    fn set_flags(&mut self, num: u8) {
        Self::write_le::<u8, SIZE_FLAGS>(&mut self.buffer, OFFSET_FLAGS, num, |value| {
            value.to_le_bytes().to_vec()
        });
    }

    fn left_most_page_id(&self) -> o16 {
        Self::read_le::<o16, SIZE_LEFT_MOST>(&self.buffer, OFFSET_LEFT_MOST, o16::from_bytes)
    }

    fn set_left_most_page_id(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_LEFT_MOST>(&mut self.buffer, OFFSET_LEFT_MOST, num, |value| {
            value.to_bytes()
        });
    }

    fn left_sibling(&self) -> o16 {
        Self::read_le::<o16, SIZE_LEFT_SIBLING>(&self.buffer, OFFSET_LEFT_SIBLING, o16::from_bytes)
    }

    fn set_left_sibling(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_LEFT_SIBLING>(
            &mut self.buffer,
            OFFSET_LEFT_SIBLING,
            num,
            |value| value.to_bytes(),
        );
    }

    fn right_sibling(&self) -> o16 {
        Self::read_le::<o16, SIZE_RIGHT_SIBLING>(
            &self.buffer,
            OFFSET_RIGHT_SIBLING,
            o16::from_bytes,
        )
    }

    fn set_right_sibling(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_RIGHT_SIBLING>(
            &mut self.buffer,
            OFFSET_RIGHT_SIBLING,
            num,
            |value| value.to_bytes(),
        );
    }

    fn parent(&self) -> o16 {
        Self::read_le::<o16, SIZE_PARENT_PAGE_ID>(
            &self.buffer,
            OFFSET_PARENT_PAGE_ID,
            o16::from_bytes,
        )
    }

    fn set_parent(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_PARENT_PAGE_ID>(
            &mut self.buffer,
            OFFSET_PARENT_PAGE_ID,
            num,
            |value| value.to_bytes(),
        );
    }

    pub(crate) fn free_start(&self) -> o16 {
        Self::read_le::<o16, SIZE_FREE_START>(&self.buffer, OFFSET_FREE_START, o16::from_bytes)
    }

    fn set_free_start(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_FREE_START>(&mut self.buffer, OFFSET_FREE_START, num, |value| {
            value.to_bytes()
        });
    }

    fn free_end(&self) -> o16 {
        Self::read_le::<o16, SIZE_FREE_END>(&self.buffer, OFFSET_FREE_END, o16::from_bytes)
    }

    fn set_free_end(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_FREE_END>(&mut self.buffer, OFFSET_FREE_END, num, |value| {
            value.to_bytes()
        });
    }
}

#[test]
fn test_add_slot_results_in_correct_num_of_slots() {
    let mut new_inner = Page::new_inner();
    let key1 = Payload::from_u16(123);
    let key2 = Payload::from_u16(789);
    let _ = new_inner.add_key_ref(Key::from_str("abc".to_string()), key1);
    let _ = new_inner.add_key_ref(Key::from_str("xyz".to_string()), key2);
    assert_eq!(new_inner.num_of_slots(), o16(2));
}

#[test]
fn verify_available_space_empty_page() -> Result<(), InvalidPageOffsetError> {
    let new_inner = Page::new_inner();
    let available_space = new_inner.free_size();
    let total_empty_size = PAGE_SIZE - TOTAL_HEADER_SIZE;
    assert_eq!(available_space, total_empty_size);
    Ok(())
}

#[test]
fn verify_available_space_after_insertion() -> Result<(), InvalidPageOffsetError> {
    let key1 = Key::from_str("foo".to_string());
    let key2 = Key::from_str("foo".to_string());
    let payload = Payload::from_str("123".to_string());
    let payload_len = payload.len();
    let mut new_inner = Page::new_inner();
    let _ = new_inner.add_key_ref(key1.clone(), payload.clone());
    let _ = new_inner.add_key_ref(key2, payload);
    let available_space: usize = new_inner.free_size().try_into()?;
    let slot_size: usize = Page::slot_size(key1.len(), payload_len).try_into()?;
    let page_size: usize = PAGE_SIZE.try_into()?;
    let total_empty_size: usize =
        page_size - (TOTAL_HEADER_SIZE + (2 * SIZE_OF_SLOT_TABLE_ITEM) + (2 * slot_size));
    assert_eq!(available_space, total_empty_size);
    Ok(())
}

#[test]
fn verify_read_the_inserted() {
    let mut new_inner = Page::new_inner();
    let payload1 = Payload::from_str("123".to_string());
    let payload2 = Payload::from_str("234".to_string());
    let _ = new_inner.add_key_ref(Key::from_str("abcdefg".to_string()), payload1);
    let _ = new_inner.add_key_ref(Key::from_str("xyz".to_string()), payload2);
    match new_inner.get_key_payload(o16(0)) {
        Ok(payload) => {
            assert_eq!(payload, "123");
        }
        Err(_) => assert!(false),
    }

    match new_inner.get_key_payload(o16(1)) {
        Ok(payload) => {
            assert_eq!(payload, "234");
        }
        Err(_) => assert!(false),
    }
}

#[test]
fn verify_add_data_node_less_than_page_size() -> Result<(), InvalidPageOffsetError> {
    let page_size: usize = PAGE_SIZE.try_into()?;
    let string = random_string(100);
    assert!(string.len() < page_size);
    let data_node = Page::new_leaf(Key::from_str("foo".to_string()), Payload::from_str(string))?;
    let cache = CACHE.lock().unwrap();
    let leading_page = cache.get(&data_node).expect("");
    assert!(leading_page.free_end() > leading_page.free_start());
    Ok(())
}

#[test]
fn verify_add_data_node_full_page() -> Result<(), InvalidPageOffsetError> {
    let key = Key::from_str("foo".to_string());
    let page_size: usize = PAGE_SIZE.try_into()?;
    let available_bytes = page_size
        - TOTAL_HEADER_SIZE
        - (2 * SIZE_PAGE_ID + 2 * SIZE_FLAGS + SIZE_OF_SLOT_TABLE_ITEM + key.len());
    let string = random_string(available_bytes);
    assert!(string.len() < page_size);
    let data_node = Page::new_leaf(key, Payload::from_str(string))?;
    let cache = CACHE.lock().unwrap();
    let leading_page = cache.get(&data_node).expect("");
    assert_eq!(leading_page.free_end(), leading_page.free_start());
    Ok(())
}

#[test]
fn verify_add_data_node_more_than_page_size() -> Result<(), InvalidPageOffsetError> {
    let page_size: usize = PAGE_SIZE.try_into()?;
    let input_value = random_string(page_size * 2);
    assert!(input_value.len() > page_size);
    let data_node = Page::new_leaf(
        Key::from_str("foo".to_string()),
        Payload::from_str(input_value.clone()),
    )?;

    let leading_page = {
        let cache = CACHE.lock().unwrap();
        cache.get(&data_node).cloned().expect("")
    };
    if let Ok(payload) = leading_page.get_key_payload(o16(0)) {
        assert_eq!(input_value, payload)
    }
    Ok(())
}

fn random_string(len: usize) -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let s: String = (0..len)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    s
}
