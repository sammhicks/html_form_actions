#![cfg_attr(not(test), no_std)]

mod tests;

pub use html_form_actions_macros::actions;

pub fn query_action(raw_query: Option<&str>) -> Option<&str> {
    raw_query?
        .split('&')
        .find(|entry| !entry.contains('=') && entry.starts_with('/'))
}

/// A helper trait for composing routers with generated routes.
pub trait BuildExt: Sized {
    /// Apply `item` to the current router, returning the new router.
    fn with<U>(self, item: impl FnOnce(Self) -> U) -> U {
        item(self)
    }
}

impl<T: Sized> BuildExt for T {}
