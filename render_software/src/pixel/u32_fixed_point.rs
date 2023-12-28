/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use std::ops::*;

use super::alpha_blend_trait::*;

///
/// Indicates a fixed point value stored in a u32
///
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub struct U32FixedPoint(pub u32);

impl U32FixedPoint {
    #[inline]
    pub(crate) fn from_u32_slice(slice: [u32; 4]) -> [U32FixedPoint; 4] {
        use std::mem;

        unsafe { mem::transmute(slice) }
    }

    #[inline]
    pub(crate) fn to_u32_slice(slice: [U32FixedPoint; 4]) -> [u32; 4] {
        use std::mem;

        unsafe { mem::transmute(slice) }
    }
}

impl From<u32> for U32FixedPoint {
    #[inline]
    fn from(val: u32) -> Self {
        U32FixedPoint(val)
    }
}

impl From<u16> for U32FixedPoint {
    #[inline]
    fn from(val: u16) -> Self {
        U32FixedPoint(val as u32)
    }
}

impl Into<u32> for U32FixedPoint {
    #[inline]
    fn into(self) -> u32 {
        self.0
    }
}

impl AlphaValue for U32FixedPoint {
    #[inline]
    fn zero() -> U32FixedPoint {
        U32FixedPoint(0)
    }
    #[inline]
    fn one() -> U32FixedPoint {
        U32FixedPoint(65535)
    }
}

impl Add<U32FixedPoint> for U32FixedPoint {
    type Output = U32FixedPoint;

    #[inline]
    fn add(self, val: U32FixedPoint) -> U32FixedPoint {
        U32FixedPoint(self.0 + val.0)
    }
}

impl Sub<U32FixedPoint> for U32FixedPoint {
    type Output = U32FixedPoint;

    #[inline]
    fn sub(self, val: U32FixedPoint) -> U32FixedPoint {
        U32FixedPoint(self.0 - val.0)
    }
}

impl Mul<U32FixedPoint> for U32FixedPoint {
    type Output = U32FixedPoint;

    #[inline]
    fn mul(self, val: U32FixedPoint) -> U32FixedPoint {
        U32FixedPoint((self.0 * val.0) >> 16)
    }
}

impl Div<U32FixedPoint> for U32FixedPoint {
    type Output = U32FixedPoint;

    #[inline]
    fn div(self, val: U32FixedPoint) -> U32FixedPoint {
        U32FixedPoint((self.0 << 16) / val.0)
    }
}
