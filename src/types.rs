use std::ops::{Add, Sub};

pub type o16 = OffsetType<u16>;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd)]
pub struct OffsetType<T>(pub T);

impl OffsetType<u16> {
    fn of(value: i32) -> o16 {
        OffsetType(value.try_into().unwrap())
    }
}

pub const fn o16(value: u16) -> o16 {
    OffsetType(value)
}

impl<T> TryFrom<usize> for OffsetType<T>
where
    T: TryFrom<usize>,
{
    type Error = crate::errors::InvalidPageOffsetError;

    fn try_from(value: usize) -> Result<Self, crate::errors::InvalidPageOffsetError> {
        T::try_from(value)
            .map(OffsetType)
            .map_err(|_| crate::errors::InvalidPageOffsetError::OutOfRange)
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

pub(crate) trait PagePayload {
    fn to_le_bytes(&self) -> Vec<u8>;
}

impl PagePayload for &str {
    fn to_le_bytes(&self) -> Vec<u8> {
        self.as_bytes().iter().map(|&b| b).collect()
    }
}

pub (crate) trait ToLeBytes {
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

pub(crate) trait FromLeBytes {
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
