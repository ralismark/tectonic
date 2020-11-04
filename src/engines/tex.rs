// src/engines/tex.rs -- Rustic interface to the core TeX engine.
// Copyright 2017-2018 the Tectonic Project
// Licensed under the MIT License.

use std::ffi::{CStr, CString};
use std::time::SystemTime;

use super::{ExecutionState, IoEventBackend, TectonicBridgeApi};
use crate::errors::{DefinitelySame, ErrorKind, Result};
use crate::io::IoStack;
use crate::status::StatusBackend;
use crate::unstable_opts::UnstableOptions;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TexResult {
    // The Errors possibility should only occur if halt_on_error_p is false --
    // otherwise, errors get upgraded to fatals. The fourth TeX "history"
    // option, "HISTORY_FATAL_ERROR" results in an Err result, not
    // Ok(TexResult).
    Spotless,
    Warnings,
    Errors,
}

// Sigh, have to do this manually because of the Result/PartialEq conflict in errors.rs
impl DefinitelySame for TexResult {
    fn definitely_same(&self, other: &Self) -> bool {
        self == other
    }
}

#[derive(Debug)]
pub struct TexEngine {
    // One day, the engine will hold its own state. For the time being,
    // though, it's just a proxy for the global constants in the C code.
    halt_on_error: bool,
    initex_mode: bool,
    synctex_enabled: bool,
    semantic_pagination_enabled: bool,
    build_date: SystemTime,
}

impl Default for TexEngine {
    fn default() -> Self {
        TexEngine {
            halt_on_error: true,
            initex_mode: false,
            synctex_enabled: false,
            semantic_pagination_enabled: false,
            build_date: SystemTime::UNIX_EPOCH,
        }
    }
}

impl TexEngine {
    pub fn new() -> TexEngine {
        TexEngine::default()
    }

    pub fn halt_on_error_mode(&mut self, halt_on_error: bool) -> &mut Self {
        self.halt_on_error = halt_on_error;
        self
    }

    /// Configure the engine to run in "initex" mode, in which it generates a
    /// "format" file that serializes the engine state rather than a PDF
    /// document.
    pub fn initex_mode(&mut self, initex: bool) -> &mut Self {
        self.initex_mode = initex;
        self
    }

    /// Configure the engine to produce SyncTeX data.
    pub fn synctex(&mut self, synctex_enabled: bool) -> &mut Self {
        self.synctex_enabled = synctex_enabled;
        self
    }

    /// Configure the engine to use “semantic pagination”.
    ///
    /// In this mode, the TeX page builder is not run, and top-level boxes are
    /// output vertically as they are created. The output file format changes
    /// from XDV to SPX (which is admittedly quite similar). "Page breaks" can
    /// be inserted explicitly in the document, but they only have semantic
    /// (organizational) meaning, rather than affecting the document
    /// rendering.
    ///
    /// This is an essential component of the HTML output process.
    pub fn semantic_pagination(&mut self, enabled: bool) -> &mut Self {
        self.semantic_pagination_enabled = enabled;
        self
    }

    /// Sets the date and time used by the TeX engine. This affects things like
    /// LaTeX's \today command. When expecting reproducible builds, this should
    /// be set to a static value, like its default value UNIX_EPOCH.
    pub fn build_date(&mut self, date: SystemTime) -> &mut Self {
        self.build_date = date;
        self
    }

    // This function can't be generic across the IoProvider trait, for now,
    // since the global pointer that stashes the ExecutionState must have a
    // complete type.

    pub fn process(
        &mut self,
        io: &mut IoStack,
        events: &mut dyn IoEventBackend,
        status: &mut dyn StatusBackend,
        format_file_name: &str,
        input_file_name: &str,
        unstables: &UnstableOptions,
    ) -> Result<TexResult> {
        let _guard = super::ENGINE_LOCK.lock().unwrap(); // until we're thread-safe ...

        let cformat = CString::new(format_file_name)?;
        let cinput = CString::new(input_file_name)?;

        let mut state = ExecutionState::new(io, events, status);
        let bridge = TectonicBridgeApi::new(&mut state);

        // initialize globals
        let mut halt_on_error = self.halt_on_error;
        if unstables.continue_on_errors {
            halt_on_error = false; // command-line override
        }
        let v = if halt_on_error { 1 } else { 0 };
        unsafe {
            super::tt_xetex_set_int_variable(b"halt_on_error_p\0".as_ptr() as _, v);
        }

        unsafe {
            super::tt_xetex_set_int_variable(
                b"shell_escape_enabled\0".as_ptr() as _,
                unstables.shell_escape as _,
            );
            super::tt_xetex_set_int_variable(
                b"in_initex_mode\0".as_ptr() as _,
                self.initex_mode as _,
            );
            super::tt_xetex_set_int_variable(
                b"synctex_enabled\0".as_ptr() as _,
                self.synctex_enabled as _,
            );
            super::tt_xetex_set_int_variable(
                b"semantic_pagination_enabled\0".as_ptr() as _,
                self.semantic_pagination_enabled as _,
            );
        }

        unsafe {
            match super::tex_simple_main(
                &bridge,
                cformat.as_ptr(),
                cinput.as_ptr(),
                self.build_date
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .expect("invalid build date")
                    .as_secs() as libc::time_t,
            ) {
                0 => Ok(TexResult::Spotless),
                1 => Ok(TexResult::Warnings),
                2 => Ok(TexResult::Errors),
                3 => {
                    let ptr = super::tt_get_error_message();
                    let msg = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                    Err(ErrorKind::Msg(msg).into())
                }
                x => Err(ErrorKind::Msg(format!(
                    "internal error: unexpected 'history' value {}",
                    x
                ))
                .into()),
            }
        }
    }
}
