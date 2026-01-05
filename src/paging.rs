use alloc::vec::Vec;
use std::convert::TryInto;
use std::error::Error;
use std::ops::{Add, Sub};

type o16 = OffsetType<u16>;

const ZERO: o16 = OffsetType(0);

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd)]
struct OffsetType<T>(pub T);

impl OffsetType<u16> {
    fn of(value: i32) -> Self {
        OffsetType(value.try_into().unwrap())
    }
}

impl<T> TryFrom<usize> for OffsetType<T>
where
    T: TryFrom<usize>,
{
    type Error = InvalidPageOffsetError;

    fn try_from(value: usize) -> Result<Self, InvalidPageOffsetError> {
        T::try_from(value)
            .map(OffsetType)
            .map_err(|_| InvalidPageOffsetError::OutOfRange)
    }
}

impl<T> TryFrom<OffsetType<T>> for usize
where
    usize: TryFrom<T>,
{
    type Error = <usize as TryFrom<T>>::Error;

    fn try_from(value: OffsetType<T>) -> Result<Self, Self::Error> {
        usize::try_from(value.0)
    }
}

trait ToLeBytes {
    fn to_le_bytes_vec(&self) -> Vec<u8>;
}

impl ToLeBytes for u16 {
    fn to_le_bytes_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToLeBytes for u32 {
    fn to_le_bytes_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

impl ToLeBytes for o16 {
    fn to_le_bytes_vec(&self) -> Vec<u8> {
        self.to_le_bytes().to_vec()
    }
}

trait FromLeBytes {
    fn from_bytes(bytes: Vec<u8>) -> Self;
}

impl FromLeBytes for o16 {
    fn from_bytes(bytes: Vec<u8>) -> o16 {
        OffsetType(u16::from_le_bytes(bytes.try_into().unwrap()))
    }
}

impl FromLeBytes for u32 {
    fn from_bytes(bytes: Vec<u8>) -> u32 {
        u32::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl FromLeBytes for u8 {
    fn from_bytes(bytes: Vec<u8>) -> u8 {
        u8::from_le_bytes(bytes.try_into().unwrap())
    }
}

impl<T> Sub for OffsetType<T>
where
    T: Sub<Output = T>,
{
    type Output = OffsetType<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        OffsetType(self.0 - rhs.0)
    }
}

impl<T> Add for OffsetType<T>
where
    T: Add<Output = T>,
{
    type Output = OffsetType<T>;

    fn add(self, rhs: Self) -> Self::Output {
        OffsetType(self.0 + rhs.0)
    }
}

impl Add<i32> for o16 {
    type Output = o16;

    fn add(self, rhs: i32) -> Self::Output {
        let right_value: u16 = rhs.try_into().expect("overflow");
        OffsetType::<u16>(self.0 + right_value)
    }
}

impl<T> PagePayload for OffsetType<T>
where
    T: ToLeBytes,
{
    fn to_le_bytes(&self) -> Vec<u8> {
        self.0.to_le_bytes_vec()
    }
}

#[derive(Debug)]
enum InvalidPageOffsetError {
    OutOfRange,
}
// TODO this needs to be persisted in a configuration file.
static mut NEXT_PAGE_ID: o16 = OffsetType(0u16);
const PAGE_SIZE: o16 = OffsetType(4096u16);
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

pub struct SlottedPage {
    buffer: [u8; PAGE_SIZE_USIZE],
}

trait PagePayload {
    fn to_le_bytes(&self) -> Vec<u8>;
}

impl PagePayload for &str {
    fn to_le_bytes(&self) -> Vec<u8> {
        self.as_bytes().iter().map(|&b| b).collect()
    }
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

impl SlottedPage {
    fn new() -> Self {
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
        new_instance.set_page_type(0);
        unsafe {
            new_instance.set_page_id(NEXT_PAGE_ID);
            NEXT_PAGE_ID = OffsetType(NEXT_PAGE_ID.0 + 1);
        }
        new_instance
    }

    pub fn new_leaf() -> Self {
        let mut instance = Self::new();
        instance.set_page_type(0);
        instance
    }

    pub fn new_inner() -> Self {
        let mut instance = Self::new();
        instance.set_page_type(1);
        instance
    }

    pub fn add_left_most(&mut self, left_most_page_id: o16) {
        self.set_left_most_page_id(left_most_page_id);
    }

    fn add_key_ref<T: PagePayload>(
        &mut self,
        key: &str,
        payload: T,
    ) -> Result<(), InvalidPageOffsetError> {
        let key_in_bytes = key.as_bytes();
        let payload_in_bytes = payload.to_le_bytes();
        let payload_len: o16 = payload_in_bytes.len().try_into()?;
        let key_in_bytes_len: o16 = key_in_bytes.len().try_into()?;
        let mut slot: Vec<u8> =
            Vec::with_capacity(Self::slot_size::<T>(key, payload).try_into().expect(""));

        slot.extend_from_slice(&payload_len.to_le_bytes());
        slot.extend_from_slice(&key_in_bytes_len.to_le_bytes());
        slot.extend_from_slice(key_in_bytes);
        slot.extend_from_slice(&payload_in_bytes);

        let required_space = slot.len() + SIZE_OF_SLOT_TABLE_ITEM;
        if self.available_size() < required_space.try_into().expect("Too many pages") {
            return Err(InvalidPageOffsetError::OutOfRange);
        }
        let new_free_end = self.add_slot(&mut slot);
        // advance the free start and slot table with the new free end.
        self.add_to_slot_table(new_free_end);
        Ok(())
    }

    fn slot_size<T: PagePayload>(key: &str, payload: T) -> o16 {
        (2 * size_of::<o16>() + key.as_bytes().len() + payload.to_le_bytes().len())
            .try_into()
            .expect("Too many pages")
    }

    fn add_to_slot_table(&mut self, new_free_end: o16) {
        let free_start = self.free_start();
        let new_free_end_offset = &new_free_end.to_le_bytes();

        let start: usize = free_start.try_into().expect("");
        let end: usize = start + SIZE_OF_SLOT_TABLE_ITEM;
        self.buffer[start..end].copy_from_slice(new_free_end_offset);

        let size_of_slot_table_item: o16 = SIZE_OF_SLOT_TABLE_ITEM.try_into().expect("");
        self.set_free_start(free_start + size_of_slot_table_item);
        self.set_num_of_slots(self.num_of_slots() + 1);
        debug_assert!(self.free_start() <= self.free_end());
    }

    fn get_key_payload(&self, index: o16) -> Result<(String, String), Box<dyn Error>> {
        let index_usize: usize = index.try_into()?;
        let o16_size: usize = size_of::<o16>().try_into()?;
        let offset_index = TOTAL_HEADER_SIZE + (index_usize * o16_size);

        let slot_offset = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            offset_index,
            o16::from_bytes,
        );

        let payload_len = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            slot_offset.try_into().expect(""),
            o16::from_bytes,
        );

        let slot_offset_usize: usize = slot_offset.try_into()?;
        let key_len = Self::read_le::<o16, SIZE_OF_SLOT_TABLE_ITEM>(
            &self.buffer,
            slot_offset_usize + SIZE_OF_SLOT_TABLE_ITEM,
            o16::from_bytes,
        );

        let key_len_usize: usize = key_len.try_into()?;
        let key = Self::read_le_into_buffer::<String>(
            &self.buffer,
            slot_offset_usize + (2 * SIZE_OF_SLOT_TABLE_ITEM),
            key_len_usize,
            |b| String::from_utf8_lossy(b.as_slice()).to_string(),
        );

        let payload = Self::read_le_into_buffer::<String>(
            &self.buffer,
            (slot_offset_usize + (2 * SIZE_OF_SLOT_TABLE_ITEM) + key_len_usize),
            payload_len.try_into().expect(""),
            |b| String::from_utf8_lossy(b.as_slice()).to_string(),
        );

        Ok((key, payload))
    }

    fn add_slot(&mut self, slot: &Vec<u8>) -> o16 {
        let free_end = self.free_end();
        let new_free_end = free_end - slot.len().try_into().expect("");
        // update the buffer with key-payload.
        self.buffer[new_free_end.try_into().expect("")..free_end.try_into().expect("")]
            .copy_from_slice(&slot);
        self.set_free_end(new_free_end);
        debug_assert!(self.free_start() <= self.free_end());
        // As we reverse traverse the slot blocks, the old free_end becomes the start of the slot.
        new_free_end
    }

    pub(crate) fn available_size(&self) -> o16 {
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
            |value| value.to_le_bytes_vec(),
        );
    }

    fn page_id(&self) -> o16 {
        Self::read_le::<o16, SIZE_PAGE_ID>(&self.buffer, OFFSET_PAGE_ID, |v| o16::from_bytes(v))
    }

    fn set_page_id(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_PAGE_ID>(&mut self.buffer, OFFSET_PAGE_ID, num, |value| {
            value.to_le_bytes_vec()
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
            value.to_le_bytes_vec()
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
            |value| value.to_le_bytes_vec(),
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
            |value| value.to_le_bytes_vec(),
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
            |value| value.to_le_bytes_vec(),
        );
    }

    pub(crate) fn free_start(&self) -> o16 {
        Self::read_le::<o16, SIZE_FREE_START>(&self.buffer, OFFSET_FREE_START, o16::from_bytes)
    }

    fn set_free_start(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_FREE_START>(&mut self.buffer, OFFSET_FREE_START, num, |value| {
            value.to_le_bytes_vec()
        });
    }

    fn free_end(&self) -> o16 {
        Self::read_le::<o16, SIZE_FREE_END>(&self.buffer, OFFSET_FREE_END, o16::from_bytes)
    }

    fn set_free_end(&mut self, num: o16) {
        Self::write_le::<o16, SIZE_FREE_END>(&mut self.buffer, OFFSET_FREE_END, num, |value| {
            value.to_le_bytes_vec()
        });
    }

    pub fn print(&self) {
        let content_as_lossy = String::from_utf8_lossy(&self.buffer);
    }
}

#[test]
fn test_add_slot_results_in_correct_num_of_slots() {
    let mut new_inner = SlottedPage::new_inner();
    new_inner.add_key_ref("abc", OffsetType::of(123));
    new_inner.add_key_ref("xyz", OffsetType::of(789));
    assert_eq!(new_inner.num_of_slots(), OffsetType::of(2));
}

#[test]
fn verify_available_space_empty_page() {
    let mut new_inner = SlottedPage::new_inner();
    let available_space = new_inner.available_size();
    let total_empty_size = PAGE_SIZE - TOTAL_HEADER_SIZE.try_into().expect("too large page size");
    assert_eq!(available_space, total_empty_size);
}

#[test]
fn verify_available_space_after_insertion() {
    let key = "abc";
    let payload = "123";
    let mut new_inner = SlottedPage::new_inner();
    new_inner.add_key_ref(key, payload);
    new_inner.add_key_ref(key, payload);
    let available_space: usize = new_inner
        .available_size()
        .try_into()
        .expect("too large page size");
    let slot_size: usize = SlottedPage::slot_size::<&str>(key, payload)
        .try_into()
        .expect("too large page size");
    let page_size: usize = PAGE_SIZE.try_into().expect("too large page size");
    let total_empty_size: usize =
        page_size - (TOTAL_HEADER_SIZE + (2 * SIZE_OF_SLOT_TABLE_ITEM) + (2 * slot_size));
    assert_eq!(available_space, total_empty_size);
}

#[test]
fn verify_read_the_inserted() {
    let mut new_inner = SlottedPage::new_inner();
    new_inner.add_key_ref("abcdefg", "123");
    new_inner.add_key_ref("xyz", "234");
    match new_inner.get_key_payload(OffsetType::of(0)) {
        Ok((key, payload)) => {
            assert_eq!(key, "abcdefg");
            assert_eq!(payload, "123");
        }
        Err(_) => assert!(false),
    }

    match new_inner.get_key_payload(OffsetType(1)) {
        Ok((key, payload)) => {
            assert_eq!(key, "xyz");
            assert_eq!(payload, "234");
        }
        Err(_) => assert!(false),
    }
}
