//! 预热模块 - 消除首次调用的延迟
//!
//! 首次调用解析函数时会有额外延迟（10-50ms），原因：
//! 1. rayon 线程池初始化
//! 2. Lazy 静态变量（SIMD Finder）初始化
//!
//! 调用 `warmup_parser()` 可以预先初始化所有组件，消除首次解析的延迟。

use std::sync::atomic::{AtomicBool, Ordering};

/// 预热状态标记
static WARMED_UP: AtomicBool = AtomicBool::new(false);

/// 预热解析器
///
/// 建议在程序启动时调用，消除首次解析的延迟（约 10-50ms）
///
/// # 示例
/// ```rust
/// use sol_parser_sdk::warmup_parser;
///
/// fn main() {
///     // 启动时预热
///     warmup_parser();
///     
///     // 后续解析将没有初始化延迟
/// }
/// ```
#[inline]
pub fn warmup_parser() {
    if WARMED_UP.swap(true, Ordering::SeqCst) {
        return; // 已经预热过了
    }

    // 1. 预热 rayon 线程池
    warmup_rayon();

    // 2. 预热所有 Lazy 静态 SIMD Finder
    warmup_simd_finders();

    // 3. 预热 Base64 引擎
    warmup_base64();
}

/// 预热 rayon 线程池
#[inline]
fn warmup_rayon() {
    // 执行一个简单的并行任务来初始化线程池
    rayon::join(|| {}, || {});
}

/// 预热所有 SIMD Finder
#[inline]
fn warmup_simd_finders() {
    use memchr::memmem;

    // 触发 optimized_matcher 中的所有 Lazy Finder 初始化
    // 通过访问它们的内部数据来强制初始化

    // 使用一个虚拟日志来触发所有 Finder
    let dummy_log = b"Program data: Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]";

    // 预热 logs/optimized_matcher.rs 中的所有 Finder
    let _ = memmem::find(dummy_log, b"Program data: ");
    let _ = memmem::find(dummy_log, b"6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
    let _ = memmem::find(dummy_log, b"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
    let _ = memmem::find(dummy_log, b"invoke [");
    let _ = memmem::find(dummy_log, b"Program");
    let _ = memmem::find(dummy_log, b"pumpswap");
    let _ = memmem::find(dummy_log, b"PumpSwap");
    let _ = memmem::find(dummy_log, b"whirL");
    let _ = memmem::find(dummy_log, b"meteora");

    // 触发 parse_invoke_info 来预热相关 Finder
    let _ = crate::logs::optimized_matcher::parse_invoke_info(
        "Program 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P invoke [1]",
    );

    // 触发 detect_log_type 来预热类型检测
    let _ = crate::logs::optimized_matcher::detect_log_type("Program data: test");
}

/// 预热 Base64 引擎
#[inline]
fn warmup_base64() {
    use base64::Engine;

    // 解码一个小的 Base64 字符串来预热引擎
    let mut buf = [0u8; 32];
    let _ = base64::engine::general_purpose::STANDARD.decode_slice(b"AAAAAAAAAAAAAAAA", &mut buf);
}

/// 检查是否已预热
#[inline]
pub fn is_warmed_up() -> bool {
    WARMED_UP.load(Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warmup() {
        warmup_parser();
        assert!(is_warmed_up());

        // 再次调用应该是 no-op
        warmup_parser();
        assert!(is_warmed_up());
    }
}
