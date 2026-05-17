use crate::kernel_config::KernelConfig;

#[derive(Clone)]
pub struct SerializedKernel<const TB_COUNT: usize, const STORE_COUNT: usize, const LUT_COUNT: usize>
{
    pub config: KernelConfig<TB_COUNT, STORE_COUNT, LUT_COUNT>,
    pub mem: Vec<i32>,
}
