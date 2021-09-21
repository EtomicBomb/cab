use std::mem::{ManuallyDrop};
use std::borrow::Borrow;
use std::fmt::{Write, Debug};
use std::fmt;
use std::ops::Deref;
use std::hash::{Hash, Hasher};

pub struct JsonString2<const INLINE_CAP: usize> {
    len: u32, // most significant bit is 1 when we've spilled onto the heap
    data: JsonStringData<INLINE_CAP>,
}

impl<const INLINE_CAP: usize> JsonString2<INLINE_CAP> {
    const MAX_LEN: usize = 1 << 31;

    pub fn with_capacity(capacity: usize) -> JsonString2<INLINE_CAP> {
        assert!(capacity < Self::MAX_LEN);

        if capacity < INLINE_CAP {
            JsonString2 {
                len: 0,
                data: JsonStringData { inline: [0; INLINE_CAP] }
            }
        } else {
            Self::from_string(String::with_capacity(capacity))
        }
    }

    fn inline(&self) -> bool {
        self.len >> 31 == 0
    }

    pub fn len(&self) -> usize {
        (self.len & 0b_01111111_11111111_11111111_11111111) as usize
    }

    pub fn from_str(string: &str) -> JsonString2<INLINE_CAP> {
        let mut ret = JsonString2::with_capacity(string.len());
        ret.push_str(string);
        ret
    }

    pub fn from_string(string: String) -> JsonString2<INLINE_CAP> {
        let mut vec = ManuallyDrop::new(string.into_bytes());

        assert!(vec.capacity() <= Self::MAX_LEN);
        let len = (vec.len() | 0b_10000000_00000000_00000000_00000000) as u32;

        JsonString2 {
            len,
            data: JsonStringData { heap: (vec.as_mut_ptr(), vec.capacity() as u32) }
        }
    }

    pub fn push_str(&mut self, to_add: &str) {
        if self.inline() {
            let spare_inline_capacity = INLINE_CAP - self.len();
            if to_add.len() <= spare_inline_capacity {
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        to_add.as_ptr(),
                        self.data.inline.as_mut_ptr().add(self.len()),
                        to_add.len()
                    );
                }

            } else { // spill
                let mut string = String::with_capacity(self.len()+to_add.len());
                string.push_str(self.as_str());
                string.push_str(to_add);
                *self = JsonString2::from_string(string);
            }

        } else {
            let (cap, ptr) = unsafe { self.data.heap };
            let mut string = unsafe { String::from_raw_parts(ptr as *mut u8, self.len(), cap as usize) };
            string.push_str(to_add);
            *self = JsonString2::from_string(string);
        }
    }

    pub fn push(&mut self, c: char) {
        let mut buf = [0; 4];
        let to_add = c.encode_utf8(&mut buf);
        self.push_str(to_add);
    }

    pub fn as_str(&self) -> &str {
        unsafe {
            let ptr = if self.inline() { self.data.inline.as_ptr() } else { self.data.heap.0 };
            let slice = std::slice::from_raw_parts(ptr, self.len());
            std::str::from_utf8_unchecked(slice)
        }
    }
}

impl<const INLINE_CAP: usize> Drop for JsonString2<INLINE_CAP> {
    fn drop(&mut self) {
        if !self.inline() {
            let (ptr, capacity) = unsafe { self.data.heap };
            unsafe { String::from_raw_parts(ptr, self.len(), capacity as usize) };
        }
    }
}

impl<const INLINE_CAP: usize> PartialEq<str> for JsonString2<INLINE_CAP> {
    fn eq(&self, other: &str) -> bool {
        (**self).eq(other)
    }
}

impl<const INLINE_CAP: usize> From<String> for JsonString2<INLINE_CAP> {
    fn from(string: String) -> JsonString2<INLINE_CAP> {
        JsonString2::from_string(string)
    }
}

impl<const INLINE_CAP: usize> From<&str> for JsonString2<INLINE_CAP> {
    fn from(string: &str) -> JsonString2<INLINE_CAP> {
        JsonString2::from_str(string)
    }
}

impl<const INLINE_CAP: usize> Borrow<str> for JsonString2<INLINE_CAP> {
    fn borrow(&self) -> &str {
        &**self
    }
}

impl<const INLINE_CAP: usize> Debug for JsonString2<INLINE_CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&**self)
    }
}

impl<const INLINE_CAP: usize> Deref for JsonString2<INLINE_CAP> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const INLINE_CAP: usize> Hash for JsonString2<INLINE_CAP> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        (**self).hash(hasher);
    }
}

union JsonStringData<const CAP: usize> {
    inline: [u8; CAP],
    heap: (*mut u8, u32),
}