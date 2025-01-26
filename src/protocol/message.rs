use lean_sys::*;
pub trait Message: std::fmt::Debug {
    fn get_round(&self) -> usize;
    unsafe fn from_lean(msg_lean: *mut lean_object, dec_refcount: bool) -> Self;
    unsafe fn to_lean(self) -> *mut lean_object;
}
