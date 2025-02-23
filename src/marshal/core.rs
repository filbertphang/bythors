use lean_sys::*;

pub const VOID_PTR_SIZE: usize = size_of::<*mut std::ffi::c_void>();

// Decrement the reference count, conditionally.
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

pub unsafe fn lean_option_to_rust<T>(
    opt: *mut lean_object,
    convert: unsafe fn(*mut lean_object) -> T,
    dec_refcount: bool,
) -> Option<T> {
    // note: the runtime representation of lean4 options are:
    // - none: lean_box(0), which is a scalar
    // - some x: a constructor with 1 parameter, where that parameter is probably x
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
