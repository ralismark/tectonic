// src/translate/mod.rs -- Gradual conversion of C engines to Rust.
// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

pub mod stringpool;

type Utf16 = u16;
type StrNumber = i32;
type PoolPointer = i32;

const TOO_BIG_CHAR: i32 = 0x10000;

#[no_mangle]
extern "C" {

    pub static str_pool: *mut Utf16;
    pub static str_start: *mut PoolPointer;
    pub static pool_ptr: PoolPointer;
    pub static str_ptr: StrNumber;

}
