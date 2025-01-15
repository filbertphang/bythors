use lean_sys::*;

/// For crate-internal debugging only.
#[allow(dead_code)]
pub unsafe fn what_is_this(s: &str, o: *mut lean_object) {
    println!(
            "\n === what is {s}? === \n {s} is string? {}.\n {s} is ref? {}.\n {s} is ctor? {}.\n {s} is scalar? {}.\n {s} is thunk? {}.\n ====== \n",
            lean_is_string(o),
            lean_is_ref(o),
            lean_is_ctor(o),
            lean_is_scalar(o),
            lean_is_thunk(o),
        );

    if lean_is_ctor(o) {
        println!(
            "since {s} is a constructor:\n
            ctor tag: {},
            num objs: {}
            ",
            lean_ptr_tag(o),
            lean_ctor_num_objs(o),
        )
    }
}
