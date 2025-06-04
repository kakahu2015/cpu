use std::fs;
use chrono::Datelike;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use num_cpus;
use chrono::{Local, NaiveTime, Weekday};
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

/*fn parse_time(time_str: &str) -> NaiveTime {
    NaiveTime::parse_from_str(time_str, "%H:%M").expect("无法解析时间格式")
}*/
fn parse_time(time_str: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_str, "%H:%M")
        .map_err(|_| format!("时间格式错误: '{}' (应为 HH:MM 格式)", time_str))
}

fn is_work_time(config: &Config) -> bool {
    let now = Local::now();
    let current_time = now.time();
    // `parse_time` 已在 main 中校验过，这里直接解包即可
    let work_start = parse_time(&config.work_start_time)
        .expect("invalid work_start_time");
    let work_end = parse_time(&config.work_end_time)
        .expect("invalid work_end_time");
    
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

// 获取系统总内存大小（以 MB 为单位）- 假设值
/*fn get_system_total_memory() -> u64 {
    // 这里我们可以从 /proc/meminfo 中读取实际的系统内存
    // 但为简单起见，假设系统有 24GB 内存（根据您的截图）
    24 * 1024  // 24GB 转换为 MB
}*/

// 简洁且高效的 Linux 版本
fn get_system_total_memory() -> u64 {
    use std::sync::OnceLock;
    static MEMORY_MB: OnceLock<u64> = OnceLock::new();
    
    *MEMORY_MB.get_or_init(|| {
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("MemTotal:"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|kb| kb.parse::<u64>().ok())
                    .map(|kb| kb / 1024) // 转换为 MB
            })
            .unwrap_or_else(|| {
                eprintln!("警告: 无法读取系统内存，使用默认值 24GB");
                24 * 1024
            })
    })
}
    
    

// 更高效的内存管理结构体
struct MemoryManager {
    memory_blocks: Vec<Vec<u8>>,
    current_percent: f64,
    system_total_memory: u64,
    random_data: Vec<u8>,      // 预先生成的随机数据，用于填充内存
}

impl MemoryManager {
    fn new() -> Self {
        let system_total_memory = get_system_total_memory();
        
        // 生成1MB的随机数据
        let mut random_data = Vec::with_capacity(1024 * 1024);
        for i in 0..1024*1024 {
            random_data.push((i % 256) as u8);
        }
        
        MemoryManager {
            memory_blocks: Vec::new(),
            current_percent: 0.0,
            system_total_memory,
            random_data,
        }
    }

    // 将内存占用百分比转换为 MB（返回值为 MB，可考虑更名）
    fn percent_to_mb(&self, percent: f64) -> usize {
        // 计算目标内存字节数（百分比 * 总内存）
        let target_mb = (self.system_total_memory as f64 * percent / 100.0) as usize;
        
        // 预留 20% 给系统和其他程序，以避免影响系统稳定性
        let safe_mb = (target_mb as f64 * 0.8) as usize;
        
        // 确保至少保留 1GB 给系统
        let min_system_reserve = 1024;
        let max_usable_mb = (self.system_total_memory as usize).saturating_sub(min_system_reserve);
        
        safe_mb.min(max_usable_mb)
    }

    // 调整内存使用百分比 - 优化版本
    fn adjust_memory_usage(&mut self, target_percent: f64) {
        println!("调整内存使用率: {:.1}%", target_percent);
        
        // 计算目标内存使用量（MB）
        let target_mb = self.percent_to_mb(target_percent);
        println!("目标内存使用量: {} MB", target_mb);
        
        // 计算当前使用的内存（MB）
        let current_mb = self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024);
        println!("当前内存使用量: {} MB", current_mb);
        
        // 如果目标内存比当前使用的少，释放一些内存
        if target_mb < current_mb {
            let blocks_to_keep = ((target_mb as f64) / (current_mb as f64)
                * (self.memory_blocks.len() as f64))
                .ceil() as usize;
            if target_mb == 0 {
                println!("释放所有内存块");
                self.memory_blocks.clear();
                self.memory_blocks.shrink_to_fit();
            } else if blocks_to_keep < self.memory_blocks.len() {
                println!("释放内存: 从 {} 个块减少到 {} 个块", self.memory_blocks.len(), blocks_to_keep);
                self.memory_blocks.truncate(blocks_to_keep);
                self.memory_blocks.shrink_to_fit();
            }

            self.current_percent = target_percent;

            // 打印释放后的内存使用情况
            let new_mb = self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024);
            println!("释放后内存使用量: {} MB", new_mb);
            return;
        }

        // 如果需要分配更多内存
        if target_mb > current_mb {
            let additional_mb = target_mb - current_mb;
            
            // 使用较大的块以减少分配次数，提高效率
            const BLOCK_SIZE_MB: usize = 512;  // 使用更大的块
            let blocks_to_add = (additional_mb + BLOCK_SIZE_MB - 1) / BLOCK_SIZE_MB;
            
            println!("需要添加 {} 个内存块，每块 {} MB", blocks_to_add, BLOCK_SIZE_MB);
            
            // 预分配大数组
            for i in 0..blocks_to_add {
                if i % 5 == 0 || i == blocks_to_add - 1 {
                    println!("已添加 {} 个内存块中的 {} 个 ({:.1}%)...", 
                             blocks_to_add, i+1, (i+1) as f64 / blocks_to_add as f64 * 100.0);
                }
                
                // 创建一个新的内存块
                let mut block = Vec::with_capacity(BLOCK_SIZE_MB * 1024 * 1024);
                
                // 使用真实数据填充它（重要）
                // 这种方法会创建真实的内存分配，不太可能被优化掉
                let block_size = BLOCK_SIZE_MB * 1024 * 1024;
                
                // 填充内存块
                for chunk_start in (0..block_size).step_by(self.random_data.len()) {
                    let remaining = block_size - chunk_start;
                    let chunk_size = remaining.min(self.random_data.len());
                    block.extend_from_slice(&self.random_data[0..chunk_size]);
                }
                
                // 确保内存实际被分配
                for chunk_start in (0..block.len()).step_by(1024*1024) {
                    let end = (chunk_start + 1000).min(block.len());
                    // 修改部分数据以确保它被访问
                    for i in chunk_start..end {
                        block[i] = block[i].wrapping_add(1);
                    }
                }
                
                self.memory_blocks.push(block);
                
                // 短暂停顿，让系统有时间处理
                if i % 10 == 9 {
                    thread::sleep(Duration::from_millis(100));
                }
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

    // 启动一个独立线程（只创建一次），周期性访问已分配的内存，防止其被系统回收
    // 通过 Arc<Mutex<MemoryManager>> 共享内存管理器
    let mm_for_keep_alive = Arc::clone(&memory_manager);
    thread::spawn(move || {
        println!("内存保持活跃线程已启动");
        loop {
            {
                let mut manager = mm_for_keep_alive.lock().unwrap();
                for block in manager.memory_blocks.iter_mut() {
                    if !block.is_empty() {
                        // 修改首字节并读取部分数据以保持活跃
                        block[0] = block[0].wrapping_add(1);
                        let _ = block[0];
                    }
                }
            }
            // 每10秒访问一次内存
            thread::sleep(Duration::from_secs(10));
        }
    });
    
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
        
        println!("\n当前时间: {}", Local::now().format("%Y-%m-%d %H:%M:%S"));
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

fn main() -> Result<(), String> {
    // 打印开始信息
    println!("===== 系统资源调节程序启动 =====");
    println!("当前进程ID: {}", process::id());
    
    // 尝试读取配置文件
    let config_content = match fs::read_to_string("config.yml") {
        Ok(content) => content,
        Err(e) => {
            println!("无法读取配置文件: {}", e);
            println!("使用默认配置...");
            r#"
work_days: []
rest_days: []
work_start_time: "09:00"
work_end_time: "18:00"
work_cpu_usage: 70.0
rest_cpu_usage: 10.0
work_memory_usage: 70.0
rest_memory_usage: 20.0
            "#.to_string()
        }
    };
    
    /*let config: Config = match serde_yaml::from_str(&config_content) {
        Ok(config) => config,
        Err(e) => {
            println!("无法解析配置文件: {}", e);
            println!("程序退出");
            return;
        }
    };*/
   let config: Config = match serde_yaml::from_str::<Config>(&config_content) {
    Ok(mut config) => {
        // 校验时间格式
        parse_time(&config.work_start_time)
            .map_err(|e| format!("work_start_time {}", e))?;
        parse_time(&config.work_end_time)
            .map_err(|e| format!("work_end_time {}", e))?;

        config
    }
    Err(e) => {
        println!("配置文件错误: {}", e);
        return Err(format!("配置文件错误: {}", e));
    }
};
        
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

    Ok(())
}
