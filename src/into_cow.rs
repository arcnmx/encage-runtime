use std::borrow::Cow;
use std::ffi::{CStr, CString};

pub trait IntoCow<'a, B: ?Sized + ToOwned> {
    fn into_cow(self) -> Cow<'a, B>;
}

impl<'a, B: ?Sized + ToOwned> IntoCow<'a, B> for Cow<'a, B> {
    fn into_cow(self) -> Cow<'a, B> {
        self
    }
}

impl<'a> IntoCow<'a, str> for String {
    fn into_cow(self) -> Cow<'a, str> {
        Cow::Owned(self)
    }
}

impl<'a> IntoCow<'a, str> for &'a str {
    fn into_cow(self) -> Cow<'a, str> {
        Cow::Borrowed(self)
    }
}

impl<'a> IntoCow<'a, CStr> for CString {
    fn into_cow(self) -> Cow<'a, CStr> {
        Cow::Owned(self)
    }
}

impl<'a> IntoCow<'a, CStr> for &'a CString {
    fn into_cow(self) -> Cow<'a, CStr> {
        Cow::Borrowed(self)
    }
}

impl<'a> IntoCow<'a, CStr> for &'a CStr {
    fn into_cow(self) -> Cow<'a, CStr> {
        Cow::Borrowed(self)
    }
}

impl<'a, T: Clone> IntoCow<'a, [T]> for Vec<T> {
    fn into_cow(self) -> Cow<'a, [T]> {
        Cow::Owned(self)
    }
}

impl<'a, T: Clone> IntoCow<'a, [T]> for &'a [T] {
    fn into_cow(self) -> Cow<'a, [T]> {
        Cow::Borrowed(self)
    }
}
