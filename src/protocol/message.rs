use lean_sys::*;
use std::fmt::{Debug, Display};
pub trait Message:
    Debug + Display + serde::Serialize + serde::de::DeserializeOwned + std::marker::Send
{
    fn get_round(&self) -> usize;
    unsafe fn from_lean(msg_lean: *mut lean_object, dec_refcount: bool) -> Self;
    unsafe fn to_lean(self) -> *mut lean_object;
}
