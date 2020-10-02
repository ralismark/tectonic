// src/translate/stringpool.rs -- Functions to work with the string pool.
// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

use std::convert::TryInto;
use std::slice;

use super::*;

pub enum PoolString {
    Char(Utf16),
    Span(&'static [Utf16]),
}

impl PoolString {
    /// Get the string which begins at str_pool[str_start[s - 65536L]]
    pub fn from(s: StrNumber) -> Self {
        // gets (str_start[s - 65536L])
        unsafe fn str_offset(s: StrNumber) -> Option<usize> {
            let offset: usize = (s - TOO_BIG_CHAR).try_into().ok()?;
            Some(*str_start.add(offset) as _)
        }

        unsafe fn str_slice(s: StrNumber) -> Option<&'static [Utf16]> {
            let offset = str_offset(s)?;
            let len = str_offset(s + 1)? - offset;
            Some(slice::from_raw_parts(str_pool.add(offset), len))
        }

        if let Some(slice) = unsafe { str_slice(s) } {
            PoolString::Span(slice)
        } else {
            PoolString::Char(s as _)
        }
    }

    /// Get string of certain length from str_ptr (which has no inherent length)
    pub fn from_strptr_with_len(len: usize) -> Self {
        unsafe {
            let offset = *str_start.add((str_ptr - TOO_BIG_CHAR) as _) as usize;
            let slice = slice::from_raw_parts(str_pool.add(offset), len);
            PoolString::Span(slice)
        }
    }

    pub fn as_slice(&self) -> &[Utf16] {
        match self {
            PoolString::Char(s) => slice::from_ref(s),
            PoolString::Span(s) => s,
        }
    }
}

#[no_mangle]
pub extern "C" fn length(s: StrNumber) -> i32 {
    // I have no idea what these cases do and why these specific numbers are used
    if let PoolString::Span(string) = PoolString::from(s) {
        string.len() as _
    } else if s >= 32 && s < 127 {
        1
    } else if s <= 127 {
        3
    } else if s < 256 {
        4
    } else {
        8
    }
}

// Hack to not duplicate declaration with bibtex.c
#[cfg(not(cbindgen_bibtex))]
#[no_mangle]
pub extern "C" fn str_eq_str(s: StrNumber, t: StrNumber) -> bool {
    PoolString::from(s).as_slice() == PoolString::from(t).as_slice()
}
