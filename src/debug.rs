use crate::marshal::string::lean_string_to_rust;
use lean_sys::*;

/// For crate-internal debugging only. Explains what this lean object is.
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

/// Used to debug print from lean without using the IO monad.
/* How to use:
 *
 * Define an opaque extern function in lean, and wrap around it to be polymorphic.
 * ```lean4
 *  @[extern "rust_print"] opaque rust_print : String → String
 *
 *  def dbg {α : Type} [Inhabited α] (s : String) (a : α) : String × α :=
 *    (rust_print s, a)
 * ```
 *
 * If you use `rust_print` on its own and discard the string message afterwards, lean will
 * treat this as a no-op and the compiler will optimize it away, so nothing will be printed.
 * (probably some flavour of dead store optimization?)
 *
 * Hence, we take in and return the extra parameter `a` here to mess with lean's compiler heuristics
 * to ensure that `rust_print` is actually called and something gets printed.
 */
#[no_mangle]
pub extern "C" fn rust_print(s: *mut lean_object) -> *mut lean_object {
    unsafe {
        let s = lean_string_to_rust(s, false);
        println!("(lean): {s}");
    }
    s
}
