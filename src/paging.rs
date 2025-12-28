use alloc::vec::Vec;

type PageId = u32;

/// ***PSize*** is used to access the page regions, which is the offset type. Its size also
/// limits the maximum page size.
type PSize = u16;

enum PageError {
    NoSpace,
}
// TODO this needs to be persisted in a configuration file.
static mut NEXT_PAGE_ID: u32 = 0;
const PAGE_SIZE: PSize = 4096;

const SIZE_NUM_OF_SLOTS: usize = size_of::<PSize>();
const SIZE_PAGE_ID: usize = size_of::<PageId>();
const SIZE_PAGE_TYPE: usize = size_of::<u8>();
const SIZE_FLAGS: usize = size_of::<u8>();
const SIZE_LEFT_MOST: usize = size_of::<PageId>();
const SIZE_LEFT_SIBLING: usize = size_of::<PageId>();
const SIZE_RIGHT_SIBLING: usize = size_of::<PageId>();
const SIZE_PARENT_PAGE_ID: usize = size_of::<PageId>();
const SIZE_FREE_START: usize = size_of::<PSize>();
const SIZE_FREE_END: usize = size_of::<PSize>();
const SIZE_OF_SLOT_TABLE_ITEM: usize = size_of::<PSize>();

const TOTAL_HEADER_SIZE: usize = SIZE_FLAGS
    + SIZE_RIGHT_SIBLING
    + SIZE_LEFT_SIBLING
    + SIZE_LEFT_MOST
    + SIZE_PARENT_PAGE_ID
    + SIZE_PAGE_ID
    + SIZE_PAGE_TYPE
    + SIZE_NUM_OF_SLOTS;

/// Offsets in the
const OFFSET_NUM_OF_SLOTS: usize = 0;
const OFFSET_PAGE_ID: usize = OFFSET_NUM_OF_SLOTS + SIZE_NUM_OF_SLOTS;
const OFFSET_PAGE_TYPE: usize = OFFSET_PAGE_ID + SIZE_PAGE_ID;
const OFFSET_FLAGS: usize = OFFSET_PAGE_TYPE + SIZE_PAGE_TYPE;
const OFFSET_LEFT_MOST: usize = OFFSET_FLAGS + SIZE_FLAGS;
const OFFSET_LEFT_SIBLING: usize = OFFSET_LEFT_MOST + SIZE_LEFT_MOST;
const OFFSET_RIGHT_SIBLING: usize = OFFSET_LEFT_SIBLING + SIZE_LEFT_SIBLING;
const OFFSET_PARENT_PAGE_ID: usize = OFFSET_RIGHT_SIBLING + SIZE_RIGHT_SIBLING;
const OFFSET_FREE_START: usize = OFFSET_PARENT_PAGE_ID + SIZE_FREE_START;
const OFFSET_FREE_END: usize = OFFSET_FREE_START + SIZE_FREE_START;

pub struct SlottedPage {
    buffer: [u8; PAGE_SIZE as usize],
}

trait PagePayload {
    const SIZE: usize;
    fn to_le_bytes(&self) -> Vec<u8>;
}

impl PagePayload for PageId {
    const SIZE: usize = size_of::<PageId>();

    fn to_le_bytes(&self) -> Vec<u8> {
        PageId::to_le_bytes(*self).to_vec()
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
            buffer: [0u8; PAGE_SIZE as usize],
        };

        new_instance.set_flags(0);
        new_instance.set_left_most_page_id(0);
        new_instance.set_right_sibling(0);
        new_instance.set_left_sibling(0);
        new_instance.set_parent(0);
        new_instance.set_num_of_slots(0);
        new_instance.set_free_start(TOTAL_HEADER_SIZE as PSize);
        new_instance.set_free_end(PAGE_SIZE as PSize);
        new_instance.set_page_type(0);
        unsafe {
            new_instance.set_page_id(NEXT_PAGE_ID);
            NEXT_PAGE_ID += 1;
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

    pub fn add_left_most(&mut self, left_most_page_id: PageId) {
        self.set_left_most_page_id(left_most_page_id);
    }

    fn add_key_ref<T: PagePayload>(&mut self, key: &str, ref_id: T) -> Result<(), PageError> {
        let key_bytes = key.as_bytes();
        let slot_payload_len = (key_bytes.len() + size_of::<T>()) as PSize;
        let mut slot: Vec<u8> = Vec::with_capacity(Self::slot_size::<T>(key) as usize);
        //  ... < | key | payload | header | key | payload | header | <- MAX_PAGE_SIZE
        slot.extend_from_slice(key_bytes);
        slot.extend_from_slice(&ref_id.to_le_bytes());
        slot.extend_from_slice(&slot_payload_len.to_le_bytes());

        let required_space = slot.len() + SIZE_OF_SLOT_TABLE_ITEM;
        if self.available_size() < required_space as PSize {
            return Err(PageError::NoSpace);
        }
        let new_free_end = self.add_slot(&mut slot);
        // advance the free start and slot table with the new free end.
        self.add_to_slot_table(&new_free_end);
        Ok(())
    }

    fn slot_size<T: PagePayload>(key: &str) -> PSize {
        (size_of::<PSize>() + key.as_bytes().len() + size_of::<T>()) as PSize
    }

    fn add_to_slot_table(&mut self, new_free_end: &PSize) {
        let free_start = self.free_start();
        let new_free_end_offset = &new_free_end.to_le_bytes();
        self.buffer[free_start as usize..free_start as usize + SIZE_OF_SLOT_TABLE_ITEM]
            .copy_from_slice(new_free_end_offset);
        self.set_free_start(free_start + SIZE_OF_SLOT_TABLE_ITEM as PSize);
        self.set_num_of_slots(self.num_of_slots() + 1);
        debug_assert!(self.free_start() <= self.free_end());
    }

    fn add_slot(&mut self, slot: &Vec<u8>) -> PSize {
        let free_end = self.free_end();
        let new_free_end = free_end - slot.len() as PSize;
        // update the buffer with key-payload.
        self.buffer[new_free_end as usize..free_end as usize].copy_from_slice(&slot);
        self.set_free_end(new_free_end);
        debug_assert!(self.free_start() <= self.free_end());
        new_free_end
    }

    pub(crate) fn available_size(&self) -> PSize {
        self.free_end().saturating_sub(self.free_start())
    }

    fn read_le<T, const N: usize>(buf: &[u8], offset: usize, f: fn([u8; N]) -> T) -> T {
        let slice = &buf[offset..offset + N];
        let arr: [u8; N] = slice.try_into().expect("slice length mismatch");
        f(arr)
    }

    fn write_le<T, const N: usize>(buf: &mut [u8], offset: usize, value: T, f: fn(T) -> [u8; N]) {
        let bytes = f(value);
        buf[offset..offset + N].copy_from_slice(&bytes);
    }

    /// Returns the number of slots from the first two bytes in the page.
    fn num_of_slots(&self) -> PSize {
        Self::read_le::<PSize, SIZE_NUM_OF_SLOTS>(
            &self.buffer,
            OFFSET_NUM_OF_SLOTS,
            PSize::from_le_bytes,
        )
    }

    fn set_num_of_slots(&mut self, num: PSize) {
        Self::write_le::<PSize, SIZE_NUM_OF_SLOTS>(
            &mut self.buffer,
            OFFSET_NUM_OF_SLOTS,
            num,
            PSize::to_le_bytes,
        );
    }

    fn page_id(&self) -> PageId {
        Self::read_le::<PageId, SIZE_PAGE_ID>(&self.buffer, OFFSET_PAGE_ID, u32::from_le_bytes)
    }

    fn set_page_id(&mut self, num: PageId) {
        Self::write_le::<PageId, SIZE_PAGE_ID>(
            &mut self.buffer,
            OFFSET_PAGE_ID,
            num,
            PageId::to_le_bytes,
        );
    }

    fn page_type(&self) -> u8 {
        Self::read_le::<u8, SIZE_PAGE_TYPE>(&self.buffer, OFFSET_PAGE_TYPE, u8::from_le_bytes)
    }

    fn set_page_type(&mut self, num: u8) {
        Self::write_le::<u8, SIZE_PAGE_TYPE>(
            &mut self.buffer,
            OFFSET_PAGE_TYPE,
            num,
            u8::to_le_bytes,
        );
    }

    fn flags(&self) -> u8 {
        Self::read_le::<u8, SIZE_FLAGS>(&self.buffer, OFFSET_FLAGS, u8::from_le_bytes)
    }

    fn set_flags(&mut self, num: u8) {
        Self::write_le::<u8, SIZE_FLAGS>(&mut self.buffer, OFFSET_FLAGS, num, u8::to_le_bytes);
    }

    fn left_most_page_id(&self) -> PageId {
        Self::read_le::<PageId, SIZE_LEFT_MOST>(
            &self.buffer,
            OFFSET_LEFT_MOST,
            PageId::from_le_bytes,
        )
    }

    fn set_left_most_page_id(&mut self, num: PageId) {
        Self::write_le::<PageId, SIZE_LEFT_MOST>(
            &mut self.buffer,
            OFFSET_LEFT_MOST,
            num,
            PageId::to_le_bytes,
        );
    }

    fn left_sibling(&self) -> PageId {
        Self::read_le::<PageId, SIZE_LEFT_SIBLING>(
            &self.buffer,
            OFFSET_LEFT_SIBLING,
            PageId::from_le_bytes,
        )
    }

    fn set_left_sibling(&mut self, num: PageId) {
        Self::write_le::<PageId, SIZE_LEFT_SIBLING>(
            &mut self.buffer,
            OFFSET_LEFT_SIBLING,
            num,
            PageId::to_le_bytes,
        );
    }

    fn right_sibling(&self) -> PageId {
        Self::read_le::<PageId, SIZE_RIGHT_SIBLING>(
            &self.buffer,
            OFFSET_RIGHT_SIBLING,
            PageId::from_le_bytes,
        )
    }

    fn set_right_sibling(&mut self, num: PageId) {
        Self::write_le::<PageId, SIZE_RIGHT_SIBLING>(
            &mut self.buffer,
            OFFSET_RIGHT_SIBLING,
            num,
            PageId::to_le_bytes,
        );
    }

    fn parent(&self) -> PageId {
        Self::read_le::<PageId, SIZE_PARENT_PAGE_ID>(
            &self.buffer,
            OFFSET_PARENT_PAGE_ID,
            PageId::from_le_bytes,
        )
    }

    fn set_parent(&mut self, num: PageId) {
        Self::write_le::<PageId, SIZE_PARENT_PAGE_ID>(
            &mut self.buffer,
            OFFSET_PARENT_PAGE_ID,
            num,
            PageId::to_le_bytes,
        );
    }

    fn free_start(&self) -> PSize {
        Self::read_le::<PSize, SIZE_FREE_START>(
            &self.buffer,
            OFFSET_FREE_START,
            PSize::from_le_bytes,
        )
    }

    fn set_free_start(&mut self, num: PSize) {
        Self::write_le::<PSize, SIZE_FREE_START>(
            &mut self.buffer,
            OFFSET_FREE_START,
            num,
            PSize::to_le_bytes,
        );
    }

    fn free_end(&self) -> PSize {
        Self::read_le::<PSize, SIZE_FREE_END>(&self.buffer, OFFSET_FREE_END, PSize::from_le_bytes)
    }

    fn set_free_end(&mut self, num: PSize) {
        Self::write_le::<PSize, SIZE_FREE_END>(
            &mut self.buffer,
            OFFSET_FREE_END,
            num,
            PSize::to_le_bytes,
        );
    }

    pub fn print(&self) {
        let content_as_lossy = String::from_utf8_lossy(&self.buffer);
        println!("{}", content_as_lossy.to_string());
    }
}

#[test]
fn test_add_slot_results_in_correct_num_of_slots() {
    let mut new_inner = SlottedPage::new_inner();
    new_inner.add_key_ref("abc", 123 as PageId);
    new_inner.add_key_ref("xyz", 789 as PageId);
    assert_eq!(new_inner.num_of_slots(), 2);
}

#[test]
fn verify_available_space_empty_page() {
    let mut new_inner = SlottedPage::new_inner();
    let available_space = new_inner.available_size();
    let total_empty_size = PAGE_SIZE - TOTAL_HEADER_SIZE as PSize;
    assert_eq!(available_space, total_empty_size as PSize);
}

#[test]
fn verify_available_space_after_insertion() {
    let key = "abc";
    let mut new_inner = SlottedPage::new_inner();
    new_inner.add_key_ref(key, 123 as PageId);
    new_inner.add_key_ref(key, 123 as PageId);
    let available_space = new_inner.available_size();
    let total_empty_size = PAGE_SIZE
        - (TOTAL_HEADER_SIZE as PSize
            + (2 * SIZE_OF_SLOT_TABLE_ITEM as PSize)
            + (2 * SlottedPage::slot_size::<PageId>(key)));
    assert_eq!(available_space, total_empty_size as PSize);
}
