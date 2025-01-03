use crate::marshal::mode::lean_dec_cond;
use lean_sys::*;

// TODO: is there a better way that transfers ownership from rust to lean
// without de-allocating/re-allocating?

/// Copies a Rust string into Lean.
/// The Rust string will be deallocated, and re-allocated on the Lean side.
pub unsafe fn rust_string_to_lean(s: String) -> *mut lean_object {
    let c_str_s = std::ffi::CString::new(s).unwrap();
    // need to cast to *const u8, since that's the type accepted by `lean_mk_string`.
    let c_str_ptr = c_str_s.as_ptr();
    // reallocation in lean occurs here
    let c_str = lean_mk_string(c_str_ptr as *const u8);

    c_str

    // rust string `s` is freed after this block ends
}

/// Copies a Lean string into Rust.
/// A new Rust string will be allocated as a copy of the Lean string.
/// Depending on the ownership mode, the Lean string may be deallocated.
pub unsafe fn lean_string_to_rust(s: *mut lean_object, dec_refcount: bool) -> String {
    let result_c_str_ptr = lean_string_cstr(s);
    let result_c_str = std::ffi::CStr::from_ptr(result_c_str_ptr as *const i8);
    let result_str = result_c_str.to_str().unwrap().to_string();

    // free c-str on lean side, if needed
    lean_dec_cond(s, dec_refcount);

    result_str
}
