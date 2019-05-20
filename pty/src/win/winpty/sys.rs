//! A rust wrapper around winpty.dll
//! https://github.com/rprichard/winpty/blob/master/src/include/winpty.h
//! This was partially generated by bindgen and then tweaked to work
//! with the shared_library macro.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use lazy_static::lazy_static;
use shared_library::shared_library;
use std::path::Path;
use winapi::shared::minwindef::{BOOL, DWORD};
use winapi::shared::ntdef::LPCWSTR;
use winapi::um::winnt::HANDLE;

pub use ::std::os::raw::c_int;

pub const WINPTY_ERROR_SUCCESS: u32 = 0;
pub const WINPTY_ERROR_OUT_OF_MEMORY: u32 = 1;
pub const WINPTY_ERROR_SPAWN_CREATE_PROCESS_FAILED: u32 = 2;
pub const WINPTY_ERROR_LOST_CONNECTION: u32 = 3;
pub const WINPTY_ERROR_AGENT_EXE_MISSING: u32 = 4;
pub const WINPTY_ERROR_UNSPECIFIED: u32 = 5;
pub const WINPTY_ERROR_AGENT_DIED: u32 = 6;
pub const WINPTY_ERROR_AGENT_TIMEOUT: u32 = 7;
pub const WINPTY_ERROR_AGENT_CREATION_FAILED: u32 = 8;
pub const WINPTY_FLAG_CONERR: u64 = 1;
pub const WINPTY_FLAG_PLAIN_OUTPUT: u64 = 2;
pub const WINPTY_FLAG_COLOR_ESCAPES: u64 = 4;
pub const WINPTY_FLAG_ALLOW_CURPROC_DESKTOP_CREATION: u64 = 8;
pub const WINPTY_MOUSE_MODE_NONE: u32 = 0;
pub const WINPTY_MOUSE_MODE_AUTO: u32 = 1;
pub const WINPTY_MOUSE_MODE_FORCE: u32 = 2;
pub const WINPTY_SPAWN_FLAG_AUTO_SHUTDOWN: u64 = 1;
pub const WINPTY_SPAWN_FLAG_EXIT_AFTER_SHUTDOWN: u64 = 2;

pub struct winpty_error_t {}
pub struct winpty_t {}
pub struct winpty_spawn_config_t {}
pub struct winpty_config_t {}

pub type winpty_error_ptr_t = *mut winpty_error_t;
pub type winpty_result_t = DWORD;

shared_library!(WinPtyFuncs,
    pub fn winpty_error_code(err: winpty_error_ptr_t) -> winpty_result_t,
    pub fn winpty_error_msg(err: winpty_error_ptr_t) -> LPCWSTR,
    pub fn winpty_error_free(err: winpty_error_ptr_t),
    pub fn winpty_config_new(
        agentFlags: u64,
        err: *mut winpty_error_ptr_t
    ) -> *mut winpty_config_t,
    pub fn winpty_config_free(cfg: *mut winpty_config_t),
    pub fn winpty_config_set_initial_size(
        cfg: *mut winpty_config_t,
        cols: c_int,
        rows: c_int
    ),
    pub fn winpty_config_set_mouse_mode(
        cfg: *mut winpty_config_t,
        mouseMode: c_int
    ),
    pub fn winpty_config_set_agent_timeout(
        cfg: *mut winpty_config_t,
        timeoutMs: DWORD
    ),
    pub fn winpty_open(cfg: *const winpty_config_t, err: *mut winpty_error_ptr_t) -> *mut winpty_t,
    pub fn winpty_agent_process(wp: *mut winpty_t) -> HANDLE,
    pub fn winpty_conin_name(wp: *mut winpty_t) -> LPCWSTR,
    pub fn winpty_conout_name(wp: *mut winpty_t) -> LPCWSTR,
    pub fn winpty_conerr_name(wp: *mut winpty_t) -> LPCWSTR,
    pub fn winpty_spawn_config_new(
        spawnFlags: u64,
        appname: LPCWSTR,
        cmdline: LPCWSTR,
        cwd: LPCWSTR,
        env: LPCWSTR,
        err: *mut winpty_error_ptr_t
    ) -> *mut winpty_spawn_config_t,
    pub fn winpty_spawn_config_free(cfg: *mut winpty_spawn_config_t),
    pub fn winpty_spawn(
        wp: *mut winpty_t,
        cfg: *const winpty_spawn_config_t,
        process_handle: *mut HANDLE,
        thread_handle: *mut HANDLE,
        create_process_error: *mut DWORD,
        err: *mut winpty_error_ptr_t
    ) -> BOOL,
    pub fn winpty_set_size(
        wp: *mut winpty_t,
        cols: c_int,
        rows: c_int,
        err: *mut winpty_error_ptr_t
    ) -> BOOL,
    /*
    pub fn winpty_get_console_process_list(
        wp: *mut winpty_t,
        processList: *mut c_int,
        processCount: c_int,
        err: *mut winpty_error_ptr_t
    ) -> c_int,
    */
    pub fn winpty_free(wp: *mut winpty_t),
);

lazy_static! {
    pub static ref WINPTY: WinPtyFuncs =
        WinPtyFuncs::open(Path::new("winpty.dll")).expect("winpty.dll is required");
}