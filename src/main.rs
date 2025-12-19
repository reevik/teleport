const PAGE_SIZE: usize = 4096;

const SIZE_NUM_OF_SLOTS: usize = size_of::<u16>();
const SIZE_PAGE_ID: usize = size_of::<u32>();
const SIZE_PAGE_TYPE: usize = size_of::<u8>();
const SIZE_FLAGS: usize = size_of::<u8>();
const SIZE_LEFT_MOST: usize = size_of::<u32>();
const SIZE_LEFT_SIBLING: usize = size_of::<u32>();
const SIZE_RIGHT_SIBLING: usize = size_of::<u32>();
const SIZE_PARENT_PAGE_ID: usize = size_of::<u32>();

/// Offsets in the
const OFFSET_NUM_OF_SLOTS: usize = 0;
const OFFSET_PAGE_ID: usize = OFFSET_NUM_OF_SLOTS + SIZE_NUM_OF_SLOTS;
const OFFSET_PAGE_TYPE: usize = OFFSET_PAGE_ID + SIZE_PAGE_ID;
const OFFSET_FLAGS: usize = OFFSET_PAGE_TYPE + SIZE_PAGE_TYPE;
const OFFSET_LEFT_MOST: usize = OFFSET_FLAGS + SIZE_FLAGS;
const OFFSET_LEFT_SIBLING: usize = OFFSET_LEFT_MOST + SIZE_LEFT_MOST;
const OFFSET_RIGHT_SIBLING: usize = OFFSET_LEFT_SIBLING + SIZE_LEFT_SIBLING;
const OFFSET_PARENT_PAGE_ID: usize = OFFSET_RIGHT_SIBLING + SIZE_RIGHT_SIBLING;
const FREE_BEGIN: usize = 0;
const FREE_END: usize = PAGE_SIZE;

struct SlottedPage {
    buffer: [u8; PAGE_SIZE],
}

impl SlottedPage {
    fn new() -> Self {
        Self {
            buffer: [0u8; PAGE_SIZE],
        }
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
        Self::write_le::<u16, SIZE_NUM_OF_SLOTS>(&mut self.buffer, OFFSET_NUM_OF_SLOTS, num, u16::to_le_bytes);
    }

    fn page_id(&self) -> u32 {
        Self::read_le::<u32, SIZE_PAGE_ID>(&self.buffer, OFFSET_PAGE_ID, u32::from_le_bytes)
    }
}

fn main() {
    println!("Hello, world!");
}
