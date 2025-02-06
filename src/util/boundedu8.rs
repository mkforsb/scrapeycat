use std::{ops::RangeInclusive, str::FromStr};

use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedU8<const L: u8, const H: u8> {
    value: u8,
}

impl<const L: u8, const H: u8> BoundedU8<L, H> {
    pub fn get(&self) -> u8 {
        self.value
    }
}

impl<const L: u8, const H: u8> TryFrom<u8> for BoundedU8<L, H> {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if L > H {
            Err(Error::InvalidRangeError)
        } else if value < L || value > H {
            Err(Error::ValueOutOfRangeError)
        } else {
            Ok(BoundedU8 { value })
        }
    }
}

impl<const L: u8, const H: u8> FromStr for BoundedU8<L, H> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        str::parse::<u8>(s)?.try_into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpperBoundedNonZeroU8<const H: u8> {
    value: u8,
}

impl<const H: u8> UpperBoundedNonZeroU8<H> {
    pub fn get(&self) -> u8 {
        self.value
    }
}

impl<const H: u8> TryFrom<u8> for UpperBoundedNonZeroU8<H> {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == 0 || value > H {
            Err(Error::ValueOutOfRangeError)
        } else {
            Ok(UpperBoundedNonZeroU8 { value })
        }
    }
}

impl<const H: u8> FromStr for UpperBoundedNonZeroU8<H> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        str::parse::<u8>(s)?.try_into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedU8RangeInclusive<const L: u8, const H: u8> {
    range: RangeInclusive<u8>,
}

impl<const L: u8, const H: u8> BoundedU8RangeInclusive<L, H> {
    pub fn get(&self) -> RangeInclusive<u8> {
        self.range.clone()
    }
}

impl<const L: u8, const H: u8> TryFrom<RangeInclusive<u8>> for BoundedU8RangeInclusive<L, H> {
    type Error = Error;

    fn try_from(value: RangeInclusive<u8>) -> Result<Self, Self::Error> {
        if L > H || *value.start() < L || *value.end() > H {
            Err(Error::ValueOutOfRangeError)
        } else if value.start() > value.end() {
            Err(Error::InvalidRangeError)
        } else {
            Ok(BoundedU8RangeInclusive { range: value })
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: do property testing
    use super::*;

    #[test]
    fn test_bounded_u8() {
        macro_rules! ok {
            ($L:expr, $H:expr, $val:expr) => {
                let x: Result<BoundedU8<$L, $H>, Error> = $val.try_into();
                assert!(x.is_ok());
            };
        }

        macro_rules! err {
            ($L:expr, $H:expr, $val:expr) => {
                let x: Result<BoundedU8<$L, $H>, Error> = $val.try_into();
                assert!(x.is_err());
            };
        }

        err!(8, 32, 0);
        err!(8, 32, 7);
        ok!(8, 32, 8);
        ok!(8, 32, 20);
        ok!(8, 32, 32);
        err!(8, 32, 33);
        err!(8, 32, 255);
        ok!(8, 255, 255);

        // invalid (reversed) range
        err!(32, 8, 20);
    }

    #[test]
    fn test_upper_bounded_non_zero_u8() {
        macro_rules! ok {
            ($H:expr, $val:expr) => {
                let x: Result<UpperBoundedNonZeroU8<$H>, Error> = $val.try_into();
                assert!(x.is_ok());
            };
        }

        macro_rules! err {
            ($H:expr, $val:expr) => {
                let x: Result<UpperBoundedNonZeroU8<$H>, Error> = $val.try_into();
                assert!(x.is_err());
            };
        }

        err!(255, 0);
        ok!(255, 1);
        ok!(1, 1);
        err!(1, 2);
        ok!(2, 2);
        err!(2, 0);
    }

    #[test]
    fn test_bounded_u8_range_inclusive() {
        macro_rules! ok {
            ($L:expr, $H:expr, $val:expr) => {
                let x: Result<BoundedU8RangeInclusive<$L, $H>, Error> = $val.try_into();
                assert!(x.is_ok());
            };
        }

        macro_rules! err {
            ($L:expr, $H:expr, $val:expr) => {
                let x: Result<BoundedU8RangeInclusive<$L, $H>, Error> = $val.try_into();
                assert!(x.is_err());
            };
        }

        ok!(0, 255, 0..=255);
        err!(1, 255, 0..=255);
        err!(0, 254, 0..=255);
        ok!(100, 120, 101..=110);
    }
}
