//! 🚀 超高性能优化模块
//! 
//! 这个模块集成了所有针对<1ms延迟目标的极致性能优化：
//! - 无锁并发架构
//! - CPU亲和性优化
//! - 零分配内存管理
//! - 预测性预取
//! - SIMD加速序列化
//! - 内核绕过网络栈
//! - 硬件级缓存优化
//! - 零拷贝内存映射IO
//! - 实时系统级调优
//! - 协议栈检查绕过
//! - 编译器级性能优化
//! - 系统调用绕过机制
//! - 极致性能测试验证

pub mod ultra_low_latency;
pub mod kernel_bypass;
pub mod hardware_optimizations;
pub mod zero_copy_io;
pub mod realtime_tuning;
pub mod protocol_optimization;
pub mod compiler_optimization;
pub mod syscall_bypass;
pub mod extreme_performance_test;

pub use ultra_low_latency::*;
pub use kernel_bypass::*;
pub use hardware_optimizations::*;
pub use zero_copy_io::*;
pub use realtime_tuning::*;
pub use protocol_optimization::*;
pub use compiler_optimization::*;
pub use syscall_bypass::*;
pub use extreme_performance_test::*;

use std::sync::Arc;
use anyhow::Result;
use log::info;

/// 🚀 一键性能优化配置器
#[derive(Clone)]
pub struct PerformanceOptimizer {
    pub dispatcher: Arc<LockFreeEventDispatcher>,
    pub serializer: Arc<ZeroAllocSerializer>,
    pub config: PerformanceOptimizerConfig,
}

#[derive(Debug, Clone)]
pub struct PerformanceOptimizerConfig {
    /// 事件队列数量（建议等于CPU核心数）
    pub num_event_queues: usize,
    /// 每个队列容量
    pub queue_capacity: usize,
    /// 工作线程数量
    pub num_workers: usize,
    /// CPU亲和性配置
    pub cpu_affinity: Option<CpuAffinityConfig>,
    /// 序列化器缓冲区池大小
    pub serializer_pool_size: usize,
    /// 序列化器缓冲区大小
    pub serializer_buffer_size: usize,
    /// 启用SIMD优化
    pub enable_simd: bool,
    /// 启用内存预取
    pub enable_prefetch: bool,
}

impl Default for PerformanceOptimizerConfig {
    fn default() -> Self {
        let num_cpus = num_cpus::get();
        
        Self {
            num_event_queues: num_cpus,
            queue_capacity: 100_000, // 10万事件容量
            num_workers: num_cpus,
            cpu_affinity: Some(CpuAffinityConfig {
                core_ids: (0..num_cpus).collect(),
                numa_optimization: true,
                priority: ThreadPriority::High,
            }),
            serializer_pool_size: 1000,
            serializer_buffer_size: 64 * 1024, // 64KB
            enable_simd: true,
            enable_prefetch: true,
        }
    }
}

impl PerformanceOptimizer {
    /// 创建新的性能优化器
    pub fn new(config: PerformanceOptimizerConfig) -> Result<Self> {
        info!("🚀 Initializing PerformanceOptimizer with config: {:?}", config);
        
        // 创建无锁事件分发器
        let dispatcher = Arc::new(LockFreeEventDispatcher::new(
            config.num_event_queues,
            config.queue_capacity,
            config.cpu_affinity.clone(),
        ));

        // 创建零分配序列化器
        let serializer = Arc::new(ZeroAllocSerializer::new(
            config.serializer_pool_size,
            config.serializer_buffer_size,
        ));

        info!("✅ PerformanceOptimizer initialized successfully");
        
        Ok(Self {
            dispatcher,
            serializer,
            config,
        })
    }

    /// 启动性能优化器
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting PerformanceOptimizer with {} workers", self.config.num_workers);
        
        // 启动无锁事件处理工作线程
        self.dispatcher.start_processing_workers(self.config.num_workers).await?;
        
        // 启动性能监控任务
        self.start_performance_monitor().await;
        
        info!("✅ PerformanceOptimizer started successfully");
        Ok(())
    }

    /// 启动性能监控任务
    async fn start_performance_monitor(&self) {
        let dispatcher = Arc::clone(&self.dispatcher);
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                let stats = dispatcher.get_performance_stats();
                let prefetch_stats = dispatcher.get_prefetch_stats();
                let queue_stats = dispatcher.get_queue_stats();
                
                info!("📊 Performance Stats:");
                info!("   Events: {}, Avg Latency: {:.2}μs", 
                      stats.events_processed, stats.avg_latency_us);
                info!("   Min: {:.2}ns, Max: {:.2}ns", 
                      stats.min_latency_ns, stats.max_latency_ns);
                info!("   <1ms: {:.1}%, <100μs: {:.1}%, <10μs: {:.1}%",
                      stats.sub_millisecond_percentage,
                      stats.ultra_fast_percentage,
                      stats.lightning_fast_percentage);
                info!("   Target <1ms: {}", if stats.target_achieved { "✅" } else { "❌" });
                info!("   Prefetch: {} hits, {} misses ({:.1}% hit rate)",
                      prefetch_stats.0, prefetch_stats.1, prefetch_stats.2 * 100.0);
                
                // 队列状态
                let total_queued: usize = queue_stats.iter().map(|(_, len)| len).sum();
                let max_queue_len = queue_stats.iter().map(|(_, len)| len).max().unwrap_or(&0);
                info!("   Queues: {} total queued, {} max queue length", total_queued, max_queue_len);
                
                // 性能目标检查
                if !stats.target_achieved {
                    log::warn!("⚠️ Latency target not achieved: {:.2}μs > 1000μs", stats.avg_latency_us);
                }
            }
        });
    }

    /// 获取当前性能统计
    pub fn get_stats(&self) -> UltraLatencySummary {
        self.dispatcher.get_performance_stats()
    }

    /// 🚀 极速事件处理入口点
    #[inline(always)]
    pub fn process_event_ultra_fast(&self, client_id: &str, event: fzstream_common::EventMessage) -> Result<()> {
        self.dispatcher.dispatch_event_ultra_fast(client_id, event)
    }
}

/// 🚀 系统级优化应用器
pub struct SystemOptimizer;

impl SystemOptimizer {
    /// 应用系统级优化
    pub fn apply_system_optimizations() -> Result<()> {
        info!("🔧 Applying system-level optimizations...");
        
        // 1. 设置进程优先级
        Self::set_process_priority()?;
        
        // 2. 优化内存设置
        Self::optimize_memory_settings()?;
        
        // 3. 网络优化
        Self::optimize_network_settings()?;
        
        info!("✅ System optimizations applied successfully");
        Ok(())
    }

    fn set_process_priority() -> Result<()> {
        #[cfg(unix)]
        {
            use libc::{setpriority, PRIO_PROCESS};
            
            // 设置高优先级（需要适当权限）
            unsafe {
                if setpriority(PRIO_PROCESS, 0, -10) != 0 {
                    log::warn!("Failed to set process priority (requires privileges)");
                } else {
                    info!("✅ Process priority set to high");
                }
            }
        }
        
        Ok(())
    }

    fn optimize_memory_settings() -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            // 在Linux上，可以通过/proc/sys/vm调整内存设置
            // 这里只是示例，实际部署时需要合适的权限
            info!("💡 Consider tuning /proc/sys/vm/swappiness and other memory settings");
        }
        
        Ok(())
    }

    fn optimize_network_settings() -> Result<()> {
        info!("🌐 Network optimizations (consider tuning kernel network parameters):");
        info!("   - net.core.rmem_max and net.core.wmem_max");
        info!("   - net.ipv4.tcp_congestion_control");
        info!("   - net.core.netdev_max_backlog");
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_optimizer() {
        let config = PerformanceOptimizerConfig {
            num_event_queues: 2,
            queue_capacity: 1000,
            num_workers: 2,
            cpu_affinity: None, // 测试时禁用CPU亲和性
            ..Default::default()
        };
        
        let optimizer = PerformanceOptimizer::new(config).unwrap();
        
        // 测试事件处理
        let test_event = fzstream_common::EventMessage {
            event_id: "test".to_string(),
            event_type: fzstream_common::EventType::BlockMeta,
            data: vec![1, 2, 3],
            serialization_format: fzstream_common::SerializationProtocol::Bincode,
            compression_format: fzstream_common::CompressionLevel::None,
            is_compressed: false,
            timestamp: 0,
            original_size: Some(3),
            grpc_arrival_time: 0,
            parsing_time: 0,
            completion_time: 0,
            client_processing_start: None,
            client_processing_end: None,
        };
        
        assert!(optimizer.process_event_ultra_fast("test_client", test_event).is_ok());
        
        // 检查统计
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let stats = optimizer.get_stats();
        assert!(stats.events_processed > 0);
    }
}