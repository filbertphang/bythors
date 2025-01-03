use lean_sys::*;

/// Indexes into an array.
/// This increments the reference count of the indexed object, and the user
/// is responsible for decrementing it when they are done with the object.
pub unsafe fn index_lean_array(arr: *mut lean_object, idx: usize) -> *mut lean_object {
    assert!(lean_is_array(arr));

    // returns a lean object.
    // if the array is an array of scalars, e.g. a u32 array, it must first be unboxed.
    lean_array_uget(arr, idx)
}

pub unsafe fn rust_vec_to_lean_array<T>(
    vec: Vec<T>,
    convert: unsafe fn(T) -> *mut lean_object,
) -> *mut lean_object {
    // this is for creating lean arrays of primitives (USize, UInt_32, etc).
    // for lean arrays of non-primitives, see impl in `rust_string_vec_to_lean_array` below.

    // this is fairly inefficient, because we do an O(n) loop to copy each array element
    // to lean array, only to do another O(n) conversion from lean Array to lean List.
    //
    // we can probably do better by creating the lean array struct then just copying over
    // the pointer for the underlying C-array into the `data` field of the struct,
    // but lets worry about performance later.

    let vec_len = vec.len();
    let arr = lean_mk_empty_array_with_capacity(lean_box(vec_len));
    for elem in vec {
        let lean_elem: *mut lean_object = convert(elem);
        lean_array_push(arr, lean_elem);
    }
    arr
}
