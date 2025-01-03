use lean_sys::*;

pub const VOID_PTR_SIZE: usize = size_of::<*mut std::ffi::c_void>();

// Decrement the reference count, conditionally.
pub unsafe fn lean_dec_cond(o: *mut lean_object, dec_refcount: bool) {
    match dec_refcount {
        false => (),
        true => lean_dec(o),
    }
}
