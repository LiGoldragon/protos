struct Holder;
type Callback = extern "C" fn(u8);

trait Methods {
    fn method(&self);
    async fn asynchronous(&self);
    unsafe fn unsafe_method(&self);
}

impl Methods for Holder {
    fn method(&self) {}
    async fn asynchronous(&self) {}
    unsafe fn unsafe_method(&self) {}
}
