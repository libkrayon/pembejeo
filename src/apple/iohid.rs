extern crate libc;

use std::ffi::c_uint;

use core_foundation::{array::CFArray, base::{CFIndex, CFTypeRef}, data::CFData, runloop::CFRunLoopRef, string::CFString};
use libc::{c_int, c_long, c_uchar, c_void, size_t};

#[link(name = "IOKit")]
extern "C" {
    pub fn IOHIDManagerCreate(allocator: *const c_void, options: c_int) -> *mut c_void;
    pub fn IOHIDManagerSetDeviceMatchingMultiple(manager: *const c_void, array: *const c_void);
    pub fn IOHIDManagerRegisterDeviceMatchingCallback(manager: *const c_void, function: fn(*mut c_void, c_int, *mut c_void, *mut c_void), context: *mut c_void);
    pub fn IOHIDManagerRegisterDeviceRemovalCallback(manager: *const c_void, function: fn(*mut c_void, c_int, *mut c_void, *mut c_void), context: *mut c_void);

    pub fn IOHIDManagerScheduleWithRunLoop(manager: *const c_void, run_loop: CFRunLoopRef, run_loop_mode: CFString);

    pub fn IOHIDManagerOpen(manager: *mut c_void, options: c_int) -> c_int;
    pub fn IOHIDManagerClose(manager: *mut c_void, options: c_int);

    pub fn IOHIDDeviceGetProperty(device: *mut c_void, property: CFString) -> CFTypeRef;
    pub fn IOHIDDeviceRegisterInputValueCallback(device: *mut c_void, function: fn(*mut c_void, c_int, *mut c_void, *mut c_void), context: *mut c_void);
    pub fn IOHIDDeviceRegisterInputReportCallback(
        device: CFTypeRef,
        report: *mut c_uchar,
        report_size: size_t,
        callback: fn(*mut c_void, i32, *mut c_void, u32, u32, *mut u8, i32),
        context: *mut c_void,
    );
    pub fn IOHIDDeviceGetReport(device: *mut c_void, report_type: c_uint, report_id: CFIndex, report: *mut u8, report_length: *mut CFIndex) -> c_int;
    pub fn IOHIDDeviceSetReport(device: *mut c_void, report_type: c_uint, report_id: CFIndex, report: *mut u8, report_length: CFIndex) -> c_int;
    pub fn IOHIDDeviceGetNumElements(device: *mut c_void) -> CFIndex;

    pub fn IOHIDValueGetElement(value: *mut c_void) -> *mut c_void;
    pub fn IOHIDValueGetLength(value: *mut c_void) -> CFIndex;
    pub fn IOHIDValueGetIntegerValue(value: *mut c_void) -> c_long;
    pub fn IOHIDValueGetBytePtr(value: *mut c_void) -> *mut u8;

    pub fn IOHIDElementGetUsagePage(element: *mut c_void) -> c_uint;
    pub fn IOHIDElementGetUsage(element: *mut c_void) -> c_uint;
}
