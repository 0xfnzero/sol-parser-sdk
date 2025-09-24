//! 性能优化提示和内联函数

/// likely - 告诉编译器条件大概率为真
#[inline(always)]
pub fn likely(condition: bool) -> bool {
    #[cold]
    fn cold() {}

    if !condition {
        cold();
    }
    condition
}

/// unlikely - 告诉编译器条件大概率为假
#[inline(always)]
pub fn unlikely(condition: bool) -> bool {
    #[cold]
    fn cold() {}

    if condition {
        cold();
    }
    condition
}

/// 预取数据到 CPU 缓存（读优化）
#[inline(always)]
pub unsafe fn prefetch_read<T>(ptr: *const T) {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T0};
        _mm_prefetch(ptr as *const i8, _MM_HINT_T0);
    }
}

/// 预取数据到 CPU 缓存（写优化）
#[inline(always)]
pub unsafe fn prefetch_write<T>(ptr: *const T) {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::{_mm_prefetch, _MM_HINT_T1};
        _mm_prefetch(ptr as *const i8, _MM_HINT_T1);
    }
}