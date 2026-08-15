struct Holder;
type Callback = extern "C" fn(u8);

struct FunctionPointers {
    callback: extern "C" fn(u8),
    unsafe_callback: unsafe extern "C" fn(),
}

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
