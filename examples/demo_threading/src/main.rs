use demo_threading;

use std::{convert::TryInto, ffi::CString, os::raw::c_char, os::raw::c_int, ptr};

extern "C" {
    fn mainCpp(arc: c_int, argv: *const *const c_char) -> c_int;
}

fn main() {
    let args: Vec<CString> = std::env::args_os()
        .map(|string| {
            #[cfg(unix)]
            use std::os::unix::ffi::OsStrExt;

            #[cfg(windows)]
            let string = string.to_string_lossy();

            CString::new(string.as_bytes()).unwrap()
        })
        .collect();

    let mut c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();
    c_args.push(ptr::null());

    unsafe {
        mainCpp(args.len().try_into().unwrap(), c_args.as_ptr());
    }
}
