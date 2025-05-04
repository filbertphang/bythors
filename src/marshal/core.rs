use lean_sys::*;

/// This module contains a grab-bag of misc helper functions.

pub const VOID_PTR_SIZE: usize = size_of::<*mut std::ffi::c_void>();

/// Decrement the reference count, conditionally.
pub unsafe fn lean_dec_cond(o: *mut lean_object, dec_refcount: bool) {
    match dec_refcount {
        false => (),
        true => lean_dec(o),
    }
}

pub fn bool_to_u8(b: bool) -> u8 {
    match b {
        true => 1,
        false => 0,
    }
}

/// Converts a Lean Option to Rust.
/// Lean options are represented differently, depending on if they are None or Some.
/// - None: lean_box(0), which is a scalar
/// - Some x: a constructor with 1 parameter, where that parameter is x
pub unsafe fn lean_option_to_rust<T>(
    opt: *mut lean_object,
    convert: unsafe fn(*mut lean_object) -> T,
    dec_refcount: bool,
) -> Option<T> {
    let res = match lean_is_scalar(opt) {
        true => None,
        false => {
            let output_lean = lean_ctor_get(opt, 0);
            let output = convert(output_lean);
            Some(output)
        }
    };

    // conditionally decrement refcount
    lean_dec_cond(opt, dec_refcount);

    res
}

pub unsafe fn rust_option_to_lean<T>(
    opt: Option<T>,
    convert: unsafe fn(T) -> *mut lean_object,
) -> *mut lean_object {
    match opt {
        None => lean_box(0),
        Some(t) => {
            let opt_lean = lean_alloc_ctor(0, 1, 0);
            lean_ctor_set(opt_lean, 0, convert(t));
            opt_lean
        }
    }
}
