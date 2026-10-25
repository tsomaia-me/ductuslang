use super::native_arg::NativeArg;
use super::native_out::NativeOut;

pub type NativeFn = fn(&[NativeArg<'_>], &mut NativeOut<'_>);
