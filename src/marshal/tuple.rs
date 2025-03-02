use crate::marshal::core::lean_dec_cond;
use lean_sys::*;

pub unsafe fn rust_tuple_to_lean<T>(
    tup: (T, T),
    convert: unsafe fn(T) -> *mut lean_object,
) -> *mut lean_object {
    // assumes T is a non-scalar type
    let tup_lean = lean_alloc_ctor(0, 2, 0);
    lean_ctor_set(tup_lean, 0, convert(tup.0));
    lean_ctor_set(tup_lean, 1, convert(tup.1));

    tup_lean
}

pub unsafe fn lean_tuple_to_rust<T>(
    tup: *mut lean_object,
    convert: unsafe fn(*mut lean_object) -> T,
    dec_refcount: bool,
) -> (T, T) {
    let t0 = convert(lean_ctor_get(tup, 0));
    let t1 = convert(lean_ctor_get(tup, 1));

    // conditionally decrement refcount
    lean_dec_cond(tup, dec_refcount);

    (t0, t1)
}
