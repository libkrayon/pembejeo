
use core::slice;
use std::{collections::HashMap, sync::{Arc, Mutex}, thread::{self, JoinHandle}};

use core_foundation::{base::TCFType, runloop::{kCFRunLoopDefaultMode, CFRunLoopGetCurrent, CFRunLoopRef, CFRunLoopRun, CFRunLoopStop}};
use libc::{c_void, c_int};

use crate::{apple::iohid::{IOHIDManagerOpen, IOHIDManagerRegisterDeviceMatchingCallback, IOHIDManagerRegisterDeviceRemovalCallback, IOHIDManagerScheduleWithRunLoop, IOHIDManagerSetDeviceMatchingMultiple}, Event, Keyboard, Mouse};

pub struct Pembejeo {
    pub mice: Mutex<HashMap<String, Mouse>>,
    pub keyboards: Mutex<HashMap<String, Keyboard>>,

    pub events: Mutex<Vec<Event>>,
    skip_checking: Mutex<bool>,
    
    input_thread: Option<JoinHandle<()>>,

    #[cfg(target_os = "macos")]
    iohid_manager: *mut c_void,
    #[cfg(target_os = "macos")]
    input_run_loop: Arc<Mutex<Option<usize>>>,
}

impl Pembejeo {
    pub fn new() -> Result<Box<Self>, crate::Error> {
        #[cfg(target_os = "macos")]
        let res = {
            use core_foundation::string::CFString;
            let iohid_manager = create_iohid_manager()?;
            let matching_array = create_matching_array();

            // Create a Pembejeo object
            let mut res = Box::new(Pembejeo {
                mice: Mutex::new(HashMap::new()),
                keyboards: Mutex::new(HashMap::new()),

                events: Mutex::new(Vec::new()),
                skip_checking: Mutex::new(false),

                input_thread: None,

                iohid_manager,
                input_run_loop: Arc::new(Mutex::new(None)),
            });

            // Setup the matching and callbacks
            unsafe {
                IOHIDManagerSetDeviceMatchingMultiple(iohid_manager, matching_array.as_CFTypeRef() as *const c_void);

                IOHIDManagerRegisterDeviceMatchingCallback(iohid_manager, handle_device_matching_callback, res.as_mut() as *mut _ as *mut c_void);
                IOHIDManagerRegisterDeviceRemovalCallback(iohid_manager, handle_device_removal_callback, res.as_mut() as *mut _ as *mut c_void);

                let iohid_manager_usize_ref = res.iohid_manager as usize;
                let input_run_loop_clone = res.input_run_loop.clone();
                res.input_thread = Some(thread::spawn(move || {
                    let run_loop = CFRunLoopGetCurrent();
                    let iohid_manager = iohid_manager_usize_ref as *mut c_void;
                    IOHIDManagerScheduleWithRunLoop(iohid_manager, run_loop, CFString::wrap_under_get_rule(kCFRunLoopDefaultMode));
                    IOHIDManagerOpen(iohid_manager, 0x00);

                    // Set the run loop
                    {
                        let mut input_run_loop = input_run_loop_clone.lock().unwrap();
                        *input_run_loop = Some(run_loop as usize);
                    }

                    CFRunLoopRun();
                }));
            }


            res
        };

        Ok(res)
    }

    pub fn poll(&self, event: &mut Event) -> bool {
        let mut events = self.events.lock().unwrap();
        if events.len() == 0 {
            *event = Event::Empty;
            return false;
        }

        *event = events.get(0).unwrap().clone();

        // Remove the event at the top of the list
        events.remove(0);
        true
    }

    pub fn wait(&self, event: &mut Event) -> bool {
        // Make a new scope for the lock or multiple threads will hang
        {
            let mut skip = self.skip_checking.lock().unwrap();
            if *skip == true {
                *skip = false;
                return false;
            }
        }

        loop {
            let events = self.events.lock().unwrap();
            if events.len() != 0 {
                break;
            }
        }

        let mut events = self.events.lock().unwrap();
        *event = events.get(0).unwrap().clone();

        // Remove the event at the top of the list
        events.remove(0);

        let mut skip = self.skip_checking.lock().unwrap();
        if events.len() == 0 { *skip = true }

        true
    }

    pub fn push_event(&self, event: &Event) {
        let mut events = self.events.lock().unwrap();
        let mut skip  = self.skip_checking.lock().unwrap();
        
        events.push(event.clone());
        *skip = false;
    }
}

impl Drop for Pembejeo {
    fn drop(&mut self) {
        // Loop until the run loop has a value
        // We need to end the run loop before joining the thread to prevent a hang.
        loop {
            let input_run_loop = self.input_run_loop.lock().unwrap();
            if let Some(_) = *input_run_loop {
                break;
            }
        }

        // Stop the run loop
        unsafe {
            let input_run_loop_guard = self.input_run_loop.lock().unwrap();
            let input_run_loop_usize = (*input_run_loop_guard).unwrap();
            let input_run_loop = input_run_loop_usize as CFRunLoopRef;
            CFRunLoopStop(input_run_loop);
        }
        

        if let Some(thread) = self.input_thread.take() {
            thread.join().unwrap();
        }

        //unsafe { IOHIDManagerClose(self.iohid_manager, 0x00) };
    }
}

#[cfg(target_os = "macos")]
fn create_iohid_manager() -> Result<*mut c_void, crate::Error> {
    use core_foundation::base::kCFAllocatorDefault;

    use crate::apple::iohid::IOHIDManagerCreate;
    let manager = unsafe { IOHIDManagerCreate(kCFAllocatorDefault, 0x00) };
    if manager == std::ptr::null_mut() {
        return Err(crate::Error::FailedCreatingPembejeo("IOHIDManagerCreate returned nullptr!".to_string()));
    }
    Ok(manager)
}

#[cfg(target_os = "macos")]
fn create_matching_dictionary(page: u16, usage: Option<u16>) ->
    core_foundation::dictionary::CFDictionary<
    core_foundation::string::CFString,
    core_foundation::number::CFNumber>
{
    use std::str::FromStr;

    use core_foundation::{dictionary::{CFDictionary, CFMutableDictionary}, number::CFNumber, string::CFString};

    let mut dict = CFMutableDictionary::<CFString, CFNumber>::new();

    let cf_page = CFNumber::from(page as i32);
    dict.set(CFString::from_str("DeviceUsagePage").unwrap(), cf_page);

    if let Some(usage) = usage {
        let cf_usage = CFNumber::from(usage as i32);
        dict.set(CFString::from_str("DeviceUsage").unwrap(), cf_usage);
    }

    dict.to_immutable()
}

#[cfg(target_os = "macos")]
fn create_matching_array() -> 
    core_foundation::array::CFArray<
        core_foundation::dictionary::CFDictionary<
            core_foundation::string::CFString,
            core_foundation::number::CFNumber,
        >
    >
{
    use core_foundation::{array::CFArray, dictionary::CFDictionary, number::CFNumber, string::CFString};

    let mouse_dict = create_matching_dictionary(0xFF00, Some(0x0C));
    //let keyboard_dict = create_matching_dictionary(0x01, Some(0x06));
    let array: CFArray<CFDictionary<CFString, CFNumber>> = CFArray::from_CFTypes(&[mouse_dict/*, keyboard_dict*/]);

    array
}

#[cfg(target_os = "macos")]
fn handle_device_matching_callback(in_context: *mut c_void, _in_return: c_int, _sender: *mut c_void, device: *mut c_void) {
    use crate::apple::iohid::{IOHIDDeviceGetProperty, IOHIDDeviceRegisterInputReportCallback, IOHIDDeviceRegisterInputValueCallback, IOHIDDeviceSetReport};
    use core_foundation::{data::{CFData, CFDataRef}, number::{CFNumber, CFNumberRef}, string::{CFString, CFStringRef}};

    let pembejeo = unsafe { &*(in_context as *mut Pembejeo) };
    
    // Send a feature report to enable multitouch
    unsafe{
        let mut report_data = [0x02_u8, 0x01_u8, 0x01u8];
        let res = IOHIDDeviceSetReport(
            device, 
            2,
            0x02,
            report_data.as_mut_ptr(),
            report_data.len() as isize
        );
        if res != 0x00 {
            eprintln!("Failed to send a feature report!");
        } else {
            println!("Succesfully send feature report!"); 
        }
    }

    // Get the device's id
    let id = format!("0x{:x}", device as usize);

    // Get the device's usage property
    let usage = unsafe {
        let usage_ref: CFNumberRef = IOHIDDeviceGetProperty(device, CFString::from("PrimaryUsage")) as CFNumberRef;
        let usage = CFNumber::wrap_under_get_rule(usage_ref);
        usage.to_i32().unwrap() as u16
    };

    // Get the device's product id
    let vendor_id = unsafe {
        let vendor_id_ref: CFNumberRef = IOHIDDeviceGetProperty(device, CFString::from("VendorID")) as CFNumberRef;
        let vendor = CFNumber::wrap_under_get_rule(vendor_id_ref);
        vendor.to_i32().unwrap() as u16
    };

    // Get the device's product id
    let product_id = unsafe {
        let product_id_ref: CFNumberRef = IOHIDDeviceGetProperty(device, CFString::from("ProductID")) as CFNumberRef;
        let product = CFNumber::wrap_under_get_rule(product_id_ref);
        product.to_i32().unwrap() as u16
    };

    // Get the device's product name
    let product = unsafe {
        let product_ref: CFStringRef = IOHIDDeviceGetProperty(device, CFString::new("Product")) as CFStringRef;
        if product_ref != std::ptr::null() {
            let product = CFString::wrap_under_get_rule(product_ref);
            product.to_string()
        } else {
            "".to_string()
        }
        
    };

    // Get the device's manufacturer name
    let manufacturer = unsafe {
        let manufacturer_ref: CFStringRef = IOHIDDeviceGetProperty(device, CFString::new("Manufacturer")) as CFStringRef;
        if manufacturer_ref != std::ptr::null() {
            let manufacturer = CFString::wrap_under_get_rule(manufacturer_ref);
            manufacturer.to_string()
        } else {
            "".to_string()
        }
    };

    // Get the device's report descriptor
    unsafe {
        let desc_ref: CFDataRef = IOHIDDeviceGetProperty(device, CFString::new("ReportDescriptor")) as CFDataRef;
        let desc_data = CFData::wrap_under_get_rule(desc_ref);
        let desc = slice::from_raw_parts(desc_data.as_ptr(), desc_data.len() as usize);

        println!("Report Descriptor:");
        for i in 0..desc.len() {
            print!("0x{:02X} ", desc[i]);
        }
        println!();
    };

    match usage {
        // Mouse or Trackpad
        0x02 => {
            // Add the mouse to the list
            let mouse = Mouse {
                id: id.clone(),
                vender_id: vendor_id.clone(),
                product_id: product_id.clone(),
                product: product.clone(),
                manufacturer: manufacturer.clone()
            };

            let mut mice = pembejeo.mice.lock().unwrap();
            (*mice).insert(id.clone(), mouse);

        },
        // Keyboards
        0x06 => {
            let keyboard = Keyboard {
                id: id.clone(),
                vender_id: vendor_id.clone(),
                product_id: product_id.clone(),
                product: product.clone(),
                manufacturer: manufacturer.clone()
            };
            let mut keyboards = pembejeo.keyboards.lock().unwrap();
            (*keyboards).insert(id.clone(), keyboard);
        },
        _ => {}
    }

    // Setup the callbacks
    unsafe {
        IOHIDDeviceRegisterInputValueCallback(device, handle_input_value_callback, in_context);

        let report_size = 64_usize;
        let report_buffer: *mut u8 = libc::malloc(report_size) as *mut u8;
        IOHIDDeviceRegisterInputReportCallback(device, report_buffer, report_size, handle_hid_report, in_context); 
    };

    // Debugging and checking information
    println!("Device Matched:");
    println!("\tID: {}", id);
    println!("\tVendor ID: {}", vendor_id);
    println!("\tProduct ID: {}", product_id);
    println!("\tProduct: {}", product);
    println!("\tManufacturer: {}", manufacturer);
    println!();
}

#[cfg(target_os = "macos")]
fn handle_device_removal_callback(in_context: *mut c_void, _in_return: c_int, _sender: *mut c_void, device: *mut c_void) {
    use core_foundation::{number::{CFNumber, CFNumberRef}, string::CFString};
    use crate::apple::iohid::IOHIDDeviceGetProperty;

    let pembejeo = unsafe { &*(in_context as *mut Pembejeo) };

    // Get the device's id
    let id = format!("0x{:x}", device as usize);

    // Get the device's usage property
    let usage = unsafe {
        let usage_ref: CFNumberRef = IOHIDDeviceGetProperty(device, CFString::from("PrimaryUsage")) as CFNumberRef;
        let usage = CFNumber::wrap_under_get_rule(usage_ref);
        usage.to_i32().unwrap() as u16
    };

    match usage {
        // Mice or trackpads
        0x02 => {
            let mut mice = pembejeo.mice.lock().unwrap();
            let _ = (*mice).remove(&id);
        },
        // Keyboards
        0x06 => {
            let mut keyboards = pembejeo.keyboards.lock().unwrap();
            let _ = (*keyboards).remove(&id);
        },
        _ => {}
    }
}

#[cfg(target_os = "macos")]
fn handle_input_value_callback(in_context: *mut c_void, in_return: c_int, sender: *mut c_void, iohid_value: *mut c_void) {
    use crate::{apple::iohid::{IOHIDDeviceCopyReports, IOHIDDeviceGetReport, IOHIDElementGetUsage, IOHIDElementGetUsagePage, IOHIDValueGetBytePtr, IOHIDValueGetElement, IOHIDValueGetIntegerValue, IOHIDValueGetLength}, mouse, Event, MouseMotionEvent};

    // let mut report_size = 16_isize;
    // let mut input_report_buffer: [u8; 64] = [0; 64]; 

    // unsafe {
    //     let res = IOHIDDeviceGetReport(sender, 0, 63, input_report_buffer.as_mut_ptr(), &mut report_size as *mut _);
    //     if res == 0x0 {
    //         print!("Received Report INPUT: ");
    //         for i in 0..report_size {
    //             print!("{:02X} ", input_report_buffer[i as usize]);
    //         }
    //         println!();
    //     } else {
    //         println!("failed to get the input report: {:02X}", res);
    //     }
    // }

    let reports = unsafe { IOHIDDeviceCopyReports(sender) };
    println!("REPORT ARRAY SIZE: {}", reports.len());
         
        
    if in_return != 0 {
        return;
    }

    let pembejeo = unsafe { &*(in_context as *mut Pembejeo) };
    let id = format!("0x{:x}", sender as usize);

    // Get the page, usage, and value 
    let (page, usage, value) = unsafe { 
        let element = IOHIDValueGetElement(iohid_value);
        let page = IOHIDElementGetUsagePage(element);
        let usage = IOHIDElementGetUsage(element);
        let value = IOHIDValueGetIntegerValue(iohid_value);
        (page, usage, value)
    };

    match page {
        // Generic Desktop
        0x01 => {
            // Mouse moved on the X or Y axis
            if usage == 0x30 || usage == 0x31 {
                // Create a event
                let mouse_motion_event =  MouseMotionEvent {
                    device_id: id,
                    x: if usage == 0x30 { value as i16 } else { 0 },
                    y: if usage == 0x31 { value as i16 } else { 0 }
                };
                if mouse_motion_event.x != 0 || mouse_motion_event.y != 0 {
                    // Add the event to the list
                    let event = Event::MouseMotion(mouse_motion_event);
                    pembejeo.push_event(&event);
                }
            }
            else {
                println!("Generic Desktop Event but unrecognized usage!");
                println!("\tUsage: 0x{:x}", usage);
                println!("\tValue: {}", value);
            }
        },


        _ => {
            println!("Alternative page: 0x{:x}", page);
            println!("\tUsage: 0x{:x}", usage);
            println!("\tValue: {}", value);
        }
    }
}

#[cfg(target_os = "macos")]
fn handle_hid_report(
    _context: *mut c_void,
    _result: i32,
    sender: *mut c_void,
    _type: u32, _report_id: u32,
    report: *mut u8,
    report_length: i32
) {
    use core_foundation::{data::{CFData, CFDataRef}, number::{CFNumber, CFNumberRef}, string::CFString};
    use crate::apple::iohid::{IOHIDDeviceGetProperty, IOHIDDeviceGetReport};
 
    unsafe { 
        //let mut report_size = 64_isize;
        //let mut input_report_buffer: [u8; 64] = [0; 64]; 
        //let mut output_report_buffer: [u8; 64] = [0; 64]; 
        //let mut feature_report_buffer: [u8; 64] = [0; 64]; 

        println!("Received Report IN: ");
        for i in 0..report_length {
            print!("{:02X} ", *report.offset(i as isize));
        }
        println!();

        //let _ = IOHIDDeviceGetReport(sender, 0, 63, input_report_buffer.as_mut_ptr(), &mut report_size as *mut _);
        //print!("Received Report INPUT: ");
        //for i in 0..report_size {
        //    print!("{:02X} ", output_report_buffer[i as usize]);
        //}
        //println!(); 

        //let _ = IOHIDDeviceGetReport(sender, 1, 0x02, output_report_buffer.as_mut_ptr(), &mut report_size as *mut _);
        //    print!("Received Report OUTPUT: ");
        //    for i in 0..report_size {
        //        print!("{:02X} ", output_report_buffer[i as usize]);
        //    }
        //println!(); 

        //let _ = IOHIDDeviceGetReport(sender, 2, 0x02, feature_report_buffer.as_mut_ptr(), &mut report_size as *mut _);
        //    print!("Received Report INPUT: ");
        //    for i in 0..report_size {
        //        print!("{:02X} ", feature_report_buffer[i as usize]);
        //    }
        //println!();

        //let mut i = 3;
        //while (i < u8::MAX) {
        //    let mut report_buffer: [u8; 64] = [0; 64]; 
        //    let res = IOHIDDeviceGetReport(sender, 2 as u32, i, report_buffer.as_mut_ptr(), &mut report_size as *mut _);


        //    if res == 0x00 {
        //        break;
        //    }
        //    i += 1;
        //}
        //println!("Loop stopped at {}", i)

         
    }
}
