// src/translate/mod.rs -- Gradual conversion of C engines to Rust.
// Copyright 2020 the Tectonic Project
// Licensed under the MIT License.

pub mod stringpool;

use crate::errors::Result;
use std::process::Command;
pub use stringpool::PoolString;

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

fn run_system_from_str(cmd: &str) -> Result<()> {
    let mut args = cmd.split_whitespace();
    Command::new(args.next().ok_or("no command given")?)
        .args(args)
        .spawn()?
        .wait()?;

    Ok(())
}

#[no_mangle]
pub unsafe extern "C" fn run_system(len: usize) -> bool {
    let pstr = PoolString::from_strptr_with_len(len);
    let pslice = &pstr.as_slice()[0..len];

    // Might want to change this to non-lossy...
    let command = String::from_utf16_lossy(pslice);
    run_system_from_str(&command).is_ok()
}
