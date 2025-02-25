use std::fs;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use num_cpus;
use chrono::{Local, NaiveTime, Datelike, Weekday};
use std::process;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    work_days: Vec<String>,      // 指定工作日列表 格式："2025-01-01"
    rest_days: Vec<String>,      // 指定休息日列表 格式："2025-01-01"
    work_start_time: String,     // 工作开始时间 格式："09:00"
    work_end_time: String,       // 工作结束时间 格式："18:00"
    work_cpu_usage: f64,         // 工作时间 CPU 使用率（百分比）
    rest_cpu_usage: f64,         // 休息时间 CPU 使用率（百分比）
    work_memory_usage: f64,      // 工作时间内存使用率（百分比）
    rest_memory_usage: f64,      // 休息时间内存使用率（百分比）
}

fn parse_time(time_str: &str) -> NaiveTime {
    NaiveTime::parse_from_str(time_str, "%H:%M").expect("无法解析时间格式")
}

fn is_work_time(config: &Config) -> bool {
    let now = Local::now();
    let current_time = now.time();
    let work_start = parse_time(&config.work_start_time);
    let work_end = parse_time(&config.work_end_time);
    
    // 检查当前日期是否在特定工作日列表中
    let current_date = now.format("%Y-%m-%d").to_string();
    if config.work_days.contains(&current_date) {
        return current_time >= work_start && current_time < work_end;
    }
    
    // 检查是否在特定休息日列表中
    if config.rest_days.contains(&current_date) {
        return false;
    }
    
    // 如果不在特定列表中，则按周一到周五判断
    let weekday = now.weekday();
    let is_weekday = match weekday {
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri => true,
        _ => false,
    };
    
    is_weekday && current_time >= work_start && current_time < work_end
}

fn get_current_cpu_usage(config: &Config) -> f64 {
    if is_work_time(config) {
        config.work_cpu_usage
    } else {
        config.rest_cpu_usage
    }
}

fn get_current_memory_usage(config: &Config) -> f64 {
    if is_work_time(config) {
        config.work_memory_usage
    } else {
        config.rest_memory_usage
    }
}

// 使用简单版本的CPU负载函数（没有系统CPU使用率检查）
fn cpu_load(config: &Config) {
    let cycle_duration = Duration::from_millis(100);
    
    loop {
        let cpu_usage = get_current_cpu_usage(config);
        let work_ratio = cpu_usage / 100.0;
        let sleep_ratio = 1.0 - work_ratio;
        
        let work_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * work_ratio);
        let sleep_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * sleep_ratio);
        
        let start = Instant::now();
        while start.elapsed() < work_duration {
            let mut x: f64 = 0.0;
            for _ in 0..1000 {
                x += 1.0;
                x = x.sqrt();
            }
        }
        
        thread::sleep(sleep_duration);
    }
}

// 获取系统总内存大小（以 MB 为单位）
fn get_system_total_memory() -> u64 {
    // 在实际代码中，我们需要使用 sysinfo 库来获取
    // 但因为有编译错误，这里使用一个固定值
    // 假设系统有 8GB 内存
    8 * 1024  // 8GB 转换为 MB
}

// 管理内存使用的结构体
struct MemoryManager {
    memory_blocks: Vec<Vec<u8>>,
    current_percent: f64,
    system_total_memory: u64,
}

impl MemoryManager {
    fn new() -> Self {
        let system_total_memory = get_system_total_memory();
        MemoryManager {
            memory_blocks: Vec::new(),
            current_percent: 0.0,
            system_total_memory,
        }
    }

    // 将百分比转换为字节数
    fn percent_to_bytes(&self, percent: f64) -> usize {
        // 计算目标内存字节数（百分比 * 总内存）
        let target_mb = (self.system_total_memory as f64 * percent / 100.0) as usize;
        
        // 预留 20% 给系统和其他程序，以避免影响系统稳定性
        let safe_mb = (target_mb as f64 * 0.8) as usize;
        
        // 确保至少保留 100MB 给系统
        let min_system_reserve = 100;
        let max_usable_mb = (self.system_total_memory as usize).saturating_sub(min_system_reserve);
        
        safe_mb.min(max_usable_mb)
    }

    // 调整内存使用百分比
    fn adjust_memory_usage(&mut self, target_percent: f64) {
        println!("调整内存使用率: {:.1}%", target_percent);
        
        // 总系统内存（MB）
        println!("系统总内存: {} MB", self.system_total_memory);
        
        // 计算目标内存使用量（MB）
        let target_mb = self.percent_to_bytes(target_percent) / (1024 * 1024);
        println!("目标内存使用量: {} MB", target_mb);
        
        // 计算当前使用的内存（MB）
        let current_mb = self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024);
        println!("当前内存使用量: {} MB", current_mb);
        
        // 如果目标内存比当前使用的少，释放一些内存
        if target_mb < current_mb {
            let blocks_to_keep = ((target_mb as f64) / (current_mb as f64) * (self.memory_blocks.len() as f64)).ceil() as usize;
            self.memory_blocks.truncate(blocks_to_keep.max(1));
            self.current_percent = target_percent;
            
            // 打印释放后的内存使用情况
            let new_mb = self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024);
            println!("释放后内存使用量: {} MB", new_mb);
            return;
        }

        // 如果需要分配更多内存
        if target_mb > current_mb {
            let additional_mb = target_mb - current_mb;
            
            // 每次分配 10MB 的块，直到达到目标
            const BLOCK_SIZE_MB: usize = 10;
            let blocks_to_add = (additional_mb + BLOCK_SIZE_MB - 1) / BLOCK_SIZE_MB;
            
            println!("需要添加 {} 个内存块，每块 {} MB", blocks_to_add, BLOCK_SIZE_MB);
            
            for i in 0..blocks_to_add {
                if i % 10 == 0 {
                    println!("已添加 {} 个内存块...", i);
                }
                
                // 分配一个新的内存块并填充随机数据
                let block_size = BLOCK_SIZE_MB * 1024 * 1024;
                let mut block = Vec::with_capacity(block_size);
                // 实际分配内存并填充数据
                block.resize(block_size, 0);
                // 用一些数据填充它，防止被优化掉
                for i in 0..block.len() {
                    if i % 1024 == 0 {  // 只填充部分数据以提高效率
                        block[i] = (i % 256) as u8;
                    }
                }
                self.memory_blocks.push(block);
            }
            
            self.current_percent = target_percent;
            
            // 打印分配后的内存使用情况
            let new_mb = self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024);
            println!("分配后内存使用量: {} MB", new_mb);
        }
    }
}

fn memory_load(config: Arc<Mutex<Config>>, memory_manager: Arc<Mutex<MemoryManager>>) {
    // 获取进程ID用于日志记录
    let pid = process::id();
    println!("内存管理线程启动，进程ID: {}", pid);
    
    loop {
        // 获取当前应该使用的内存百分比
        let target_memory_percent = {
            let config = config.lock().unwrap();
            get_current_memory_usage(&config)
        };
        
        // 检查是否是工作时间
        let is_work = {
            let config = config.lock().unwrap();
            is_work_time(&config)
        };
        
        println!("当前状态: {}", if is_work { "工作时间" } else { "休息时间" });
        
        // 调整内存使用
        {
            let mut manager = memory_manager.lock().unwrap();
            manager.adjust_memory_usage(target_memory_percent);
        }
        
        // 每分钟检查一次是否需要调整内存
        thread::sleep(Duration::from_secs(60));
    }
}

fn main() {
    // 打印开始信息
    println!("===== 系统资源调节程序启动 =====");
    println!("当前进程ID: {}", process::id());
    
    let config_content = fs::read_to_string("config.yml")
        .expect("无法读取配置文件");
    
    let config: Config = serde_yaml::from_str(&config_content)
        .expect("无法解析配置文件");
        
    println!("\n===== 配置信息 =====");
    println!("CPU 核心数: {}", num_cpus::get());
    println!("工作时间 CPU 使用率: {:.1}%", config.work_cpu_usage);
    println!("休息时间 CPU 使用率: {:.1}%", config.rest_cpu_usage);
    println!("工作时间内存使用率: {:.1}%", config.work_memory_usage);
    println!("休息时间内存使用率: {:.1}%", config.rest_memory_usage);
    println!("工作开始时间: {}", config.work_start_time);
    println!("工作结束时间: {}", config.work_end_time);
    println!("指定工作日数量: {}", config.work_days.len());
    println!("指定休息日数量: {}", config.rest_days.len());
    
    // 检查当前是否是工作时间
    let is_work_now = is_work_time(&config);
    println!("\n当前时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    println!("当前状态: {}", if is_work_now { "工作时间" } else { "休息时间" });
    println!("当前应用 CPU 使用率: {:.1}%", get_current_cpu_usage(&config));
    println!("当前应用内存使用率: {:.1}%", get_current_memory_usage(&config));
    
    // 将配置封装在 Arc<Mutex<>> 中以便在线程间共享
    let shared_config = Arc::new(Mutex::new(config));
    
    // 创建内存管理器
    let memory_manager = Arc::new(Mutex::new(MemoryManager::new()));
    
    println!("\n===== 启动资源管理线程 =====");
    
    // 启动内存管理线程
    let memory_config = Arc::clone(&shared_config);
    let memory_mgr = Arc::clone(&memory_manager);
    let memory_handle = thread::spawn(move || {
        println!("启动内存负载线程");
        memory_load(memory_config, memory_mgr);
    });
    
    // 启动 CPU 负载线程
    let mut handles = vec![];
    for i in 0..num_cpus::get() {
        let config_clone = {
            let config = shared_config.lock().unwrap();
            config.clone()
        };
        
        let handle = thread::spawn(move || {
            println!("启动 CPU 负载线程，核心 #{}", i);
            cpu_load(&config_clone);
        });
        
        handles.push(handle);
    }
    
    println!("\n===== 所有线程已启动 =====");
    println!("程序运行中...");
    
    // 等待所有线程完成
    handles.push(memory_handle);
    for handle in handles {
        handle.join().unwrap();
    }
}
