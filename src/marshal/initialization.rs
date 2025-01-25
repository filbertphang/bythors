use lean_sys::*;

// https://lean-lang.org/lean4/doc/dev/ffi.html#initialization
// https://git.leni.sh/aniva/RustCallLean/src/branch/main/src/main.rs#L30
pub unsafe fn initialize_lean_environment(
    initialize_callee: unsafe fn(u8, lean_obj_arg) -> lean_obj_res,
) {
    lean_initialize_runtime_module();
    lean_initialize(); // necessary if you (indirectly) access the `Lean` package

    let builtin: u8 = 1;
    let res = initialize_callee(builtin, lean_io_mk_world());

    if lean_io_result_is_ok(res) {
        lean_dec_ref(res);
    } else {
        lean_io_result_show_error(res);
        lean_dec(res);
        panic!("Failed to load callee!");
    }
    //lean_init_task_manager(); // necessary if you (indirectly) use `Task`
    lean_io_mark_end_initialization();
}

// io helpers
pub unsafe fn cleanup_lean_io(o: *mut lean_object) {
    if lean_io_result_is_ok(o) {
        lean_dec_ref(o);
    } else {
        lean_io_result_show_error(o);
        lean_dec(o);
        panic!("IO Monad execution failed");
    }
}
