const PAGE_SIZE: usize = 4096;

const SIZE_NUM_OF_SLOTS: usize = size_of::<u16>();
const SIZE_PAGE_ID: usize = size_of::<u32>();
const SIZE_PAGE_TYPE: usize = size_of::<u8>();
const SIZE_FLAGS: usize = size_of::<u8>();
const SIZE_LEFT_MOST: usize = size_of::<u32>();
const SIZE_LEFT_SIBLING: usize = size_of::<u32>();
const SIZE_RIGHT_SIBLING: usize = size_of::<u32>();
const SIZE_PARENT_PAGE_ID: usize = size_of::<u32>();
const SIZE_FREE_START: usize = size_of::<u16>();
const SIZE_FREE_END: usize = size_of::<u16>();

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
    buffer: [u8; PAGE_SIZE],
}

struct Slot {
    buffer: Vec<u8>
}

impl SlottedPage {
    pub fn new_inner() -> Self {
        let mut new_instance = Self {
            buffer: [0u8; PAGE_SIZE],
        };

        new_instance.set_flags(0);
        new_instance.set_left_most_page_id(0);
        new_instance.set_right_sibling(0);
        new_instance.set_left_sibling(0);
        new_instance.set_parent(0);
        new_instance.set_num_of_slots(0);
        new_instance.set_page_type(0);
        new_instance.set_free_start(PAGE_SIZE as u16);
        new_instance.set_free_end(TOTAL_HEADER_SIZE as u16);

        new_instance
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
    fn num_of_slots(&self) -> u16 {
        Self::read_le::<u16, SIZE_NUM_OF_SLOTS>(
            &self.buffer,
            OFFSET_NUM_OF_SLOTS,
            u16::from_le_bytes,
        )
    }

    fn set_num_of_slots(&mut self, num: u16) {
        Self::write_le::<u16, SIZE_NUM_OF_SLOTS>(
            &mut self.buffer,
            OFFSET_NUM_OF_SLOTS,
            num,
            u16::to_le_bytes,
        );
    }

    fn page_id(&self) -> u32 {
        Self::read_le::<u32, SIZE_PAGE_ID>(&self.buffer, OFFSET_PAGE_ID, u32::from_le_bytes)
    }

    fn set_page_id(&mut self, num: u32) {
        Self::write_le::<u32, SIZE_PAGE_ID>(
            &mut self.buffer,
            OFFSET_PAGE_ID,
            num,
            u32::to_le_bytes,
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

    fn left_most_page_id(&self) -> u32 {
        Self::read_le::<u32, SIZE_LEFT_MOST>(&self.buffer, OFFSET_LEFT_MOST, u32::from_le_bytes)
    }

    fn set_left_most_page_id(&mut self, num: u32) {
        Self::write_le::<u32, SIZE_LEFT_MOST>(
            &mut self.buffer,
            OFFSET_LEFT_MOST,
            num,
            u32::to_le_bytes,
        );
    }

    fn left_sibling(&self) -> u32 {
        Self::read_le::<u32, SIZE_LEFT_SIBLING>(
            &self.buffer,
            OFFSET_LEFT_SIBLING,
            u32::from_le_bytes,
        )
    }

    fn set_left_sibling(&mut self, num: u32) {
        Self::write_le::<u32, SIZE_LEFT_SIBLING>(
            &mut self.buffer,
            OFFSET_LEFT_SIBLING,
            num,
            u32::to_le_bytes,
        );
    }

    fn right_sibling(&self) -> u32 {
        Self::read_le::<u32, SIZE_RIGHT_SIBLING>(
            &self.buffer,
            OFFSET_RIGHT_SIBLING,
            u32::from_le_bytes,
        )
    }

    fn set_right_sibling(&mut self, num: u32) {
        Self::write_le::<u32, SIZE_RIGHT_SIBLING>(
            &mut self.buffer,
            OFFSET_RIGHT_SIBLING,
            num,
            u32::to_le_bytes,
        );
    }

    fn parent(&self) -> u32 {
        Self::read_le::<u32, SIZE_PARENT_PAGE_ID>(
            &self.buffer,
            OFFSET_PARENT_PAGE_ID,
            u32::from_le_bytes,
        )
    }

    fn set_parent(&mut self, num: u32) {
        Self::write_le::<u32, SIZE_PARENT_PAGE_ID>(
            &mut self.buffer,
            OFFSET_PARENT_PAGE_ID,
            num,
            u32::to_le_bytes,
        );
    }

    fn free_start(&self) -> u16 {
        Self::read_le::<u16, SIZE_FREE_START>(&self.buffer, OFFSET_FREE_START, u16::from_le_bytes)
    }

    fn set_free_start(&mut self, num: u16) {
        Self::write_le::<u16, SIZE_FREE_START>(
            &mut self.buffer,
            OFFSET_FREE_START,
            num,
            u16::to_le_bytes,
        );
    }

    fn free_end(&self) -> u16 {
        Self::read_le::<u16, SIZE_FREE_END>(&self.buffer, OFFSET_FREE_END, u16::from_le_bytes)
    }

    fn set_free_end(&mut self, num: u16) {
        Self::write_le::<u16, SIZE_FREE_END>(
            &mut self.buffer,
            OFFSET_FREE_END,
            num,
            u16::to_le_bytes,
        );
    }
}
