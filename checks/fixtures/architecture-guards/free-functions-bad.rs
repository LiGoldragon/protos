pub(crate) fn crate_visible() {}
pub async fn asynchronous() {}
const fn constant() {}
unsafe fn unsafe_function() {}

mod nested {
    #[inline]
    pub(crate) async unsafe fn inline_nested() {}
}
