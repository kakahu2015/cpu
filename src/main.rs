use std::fs;
use chrono::Datelike;
use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use num_cpus;
use chrono::{Local, NaiveTime, Weekday};
use std::process;

// 常量定义
const CPU_CYCLE_DURATION_MS: u64 = 10;
const MIN_SLEEP_MICROS: u64 = 100;
const MEMORY_CHECK_INTERVAL_SECS: u64 = 60;
const MEMORY_KEEP_ALIVE_INTERVAL_SECS: u64 = 10;
const SYSTEM_RESERVE_RATIO: f64 = 0.8;
const MIN_SYSTEM_RESERVE_MB: usize = 1024;
const RANDOM_DATA_SIZE: usize = 1024 * 1024;
const MEMORY_BLOCK_SIZES: [usize; 3] = [64, 128, 256];
const BATCH_PROGRESS_INTERVAL: usize = 5;
const BATCH_PAUSE_INTERVAL: usize = 10;
const BATCH_PAUSE_DURATION_MS: u64 = 100;

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
    #[serde(skip)]
    work_start: NaiveTime,       // 解析后的工作开始时间
    #[serde(skip)]
    work_end: NaiveTime,         // 解析后的工作结束时间
}

fn parse_time(time_str: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(time_str, "%H:%M")
        .map_err(|_| format!("时间格式错误: '{}' (应为 HH:MM 格式)", time_str))
}

fn clamp_percent(value: &mut f64, name: &str) {
    if *value < 0.0 {
        eprintln!("警告: {} {:.1}% 小于 0，已截断为 0", name, *value);
        *value = 0.0;
    } else if *value > 100.0 {
        eprintln!("警告: {} {:.1}% 大于 100，已截断为 100", name, *value);
        *value = 100.0;
    }
}

fn is_work_time_at(config: &Config, now: chrono::DateTime<Local>) -> bool {
    let current_time = now.time();
    // 直接使用解析后的时间
    let work_start = config.work_start;
    let work_end = config.work_end;

    // 处理跨日班次：若结束时间早于开始时间，说明工作段跨越午夜，
    // 判断逻辑为 current_time >= work_start || current_time < work_end
    let in_time_range = if work_end >= work_start {
        current_time >= work_start && current_time < work_end
    } else {
        current_time >= work_start || current_time < work_end
    };
    
    // 检查当前日期是否在特定工作日列表中
    let current_date = now.format("%Y-%m-%d").to_string();
    if config.work_days.contains(&current_date) {
        return in_time_range;
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
    
    is_weekday && in_time_range
}

fn is_work_time(config: &Config) -> bool {
    let now = Local::now();
    is_work_time_at(config, now)
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


fn cpu_load(config: Arc<Mutex<Config>>) {
    loop {
        let cycle_start = Instant::now();
        let cycle_duration = Duration::from_millis(CPU_CYCLE_DURATION_MS);
        
        let cpu_usage = {
            let cfg = config.lock().unwrap();
            get_current_cpu_usage(&cfg)
        };
        
        let work_ratio = cpu_usage / 100.0;
        let target_work_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * work_ratio);
        
        if work_ratio > 0.0 {
            let work_start = Instant::now();
            while work_start.elapsed() < target_work_duration {
                let mut x = 1.0f64;
                for _ in 0..5000 {
                    x = x.powi(2).sqrt().sin().cos().tan().exp().ln();
                }
                std::hint::black_box(x);
            }
        }
        
        let elapsed = cycle_start.elapsed();
        let sleep_time = cycle_duration.saturating_sub(elapsed);
        
        if sleep_time > Duration::from_micros(MIN_SLEEP_MICROS) {
            thread::sleep(sleep_time);
        } else {
            thread::sleep(Duration::from_micros(MIN_SLEEP_MICROS));
        }
    }
}


    



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
        
        // 生成随机数据
        let mut random_data = Vec::with_capacity(RANDOM_DATA_SIZE);
        for i in 0..RANDOM_DATA_SIZE {
            random_data.push((i % 256) as u8);
        }
        
        MemoryManager {
            memory_blocks: Vec::new(),
            current_percent: 0.0,
            system_total_memory,
            random_data,
        }
    }

    // 将内存占用百分比转换为 MB，返回单位为 MB
    fn percent_to_mb(&self, mut percent: f64) -> usize {
        if percent < 0.0 {
            eprintln!("警告: 内存使用率 {:.1}% 小于 0，已截断为 0", percent);
            percent = 0.0;
        } else if percent > 100.0 {
            eprintln!("警告: 内存使用率 {:.1}% 大于 100，已截断为 100", percent);
            percent = 100.0;
        }

        // 计算目标内存字节数（百分比 * 总内存）
        let target_mb = (self.system_total_memory as f64 * percent / 100.0) as usize;
        
        // 预留给系统和其他程序，以避免影响系统稳定性
        let safe_mb = (target_mb as f64 * SYSTEM_RESERVE_RATIO) as usize;
        
        // 确保至少保留给系统
        let max_usable_mb = (self.system_total_memory as usize).saturating_sub(MIN_SYSTEM_RESERVE_MB);
        
        safe_mb.min(max_usable_mb)
    }

    fn adjust_memory_usage(&mut self, target_percent: f64) {
        println!("调整内存使用率: {:.1}%", target_percent);
        
        let target_mb = self.percent_to_mb(target_percent);
        let current_mb = self.get_current_memory_mb();
        
        println!("目标内存使用量: {} MB", target_mb);
        println!("当前内存使用量: {} MB", current_mb);
        
        match target_mb.cmp(&current_mb) {
            std::cmp::Ordering::Less => self.release_memory_to_target(target_mb),
            std::cmp::Ordering::Greater => self.allocate_memory_to_target(target_mb, current_mb),
            std::cmp::Ordering::Equal => {}
        }
        
        self.current_percent = target_percent;
    }

    fn get_current_memory_mb(&self) -> usize {
        self.memory_blocks.iter().map(|b| b.len()).sum::<usize>() / (1024 * 1024)
    }

    fn release_memory_to_target(&mut self, target_mb: usize) {
        while self.get_current_memory_mb() > target_mb && !self.memory_blocks.is_empty() {
            self.memory_blocks.pop();
        }
        let final_mb = self.get_current_memory_mb();
        println!("释放后内存使用量: {} MB", final_mb);
    }

    fn allocate_memory_to_target(&mut self, target_mb: usize, current_mb: usize) {
        let additional_mb = target_mb - current_mb;
        let block_size = self.calculate_block_size(additional_mb);
        let blocks_needed = (additional_mb + block_size - 1) / block_size;
        
        println!("需要添加 {} 个内存块，每块 {} MB", blocks_needed, block_size);
        
        for i in 0..blocks_needed {
            let block = self.create_memory_block(block_size);
            self.memory_blocks.push(block);
            
            if i % BATCH_PROGRESS_INTERVAL == 0 || i == blocks_needed - 1 {
                println!("已添加 {} 个内存块中的 {} 个", blocks_needed, i + 1);
            }
            
            if i % BATCH_PAUSE_INTERVAL == BATCH_PAUSE_INTERVAL - 1 {
                thread::sleep(Duration::from_millis(BATCH_PAUSE_DURATION_MS));
            }
        }
        
        let final_mb = self.get_current_memory_mb();
        println!("分配后内存使用量: {} MB", final_mb);
    }

    fn calculate_block_size(&self, additional_mb: usize) -> usize {
        if additional_mb < 256 { 
            MEMORY_BLOCK_SIZES[0] 
        } else if additional_mb < 1024 { 
            MEMORY_BLOCK_SIZES[1] 
        } else { 
            MEMORY_BLOCK_SIZES[2] 
        }
    }

    fn create_memory_block(&self, size_mb: usize) -> Vec<u8> {
        let mut block = Vec::with_capacity(size_mb * 1024 * 1024);
        let block_bytes = size_mb * 1024 * 1024;
        
        // 用真实数据填充
        for chunk_start in (0..block_bytes).step_by(self.random_data.len()) {
            let remaining = block_bytes - chunk_start;
            let chunk_size = remaining.min(self.random_data.len());
            block.extend_from_slice(&self.random_data[0..chunk_size]);
        }
        
        // 确保内存被实际使用
        for chunk_start in (0..block.len()).step_by(1024*1024) {
            let end = (chunk_start + 1000).min(block.len());
            for j in chunk_start..end {
                block[j] = block[j].wrapping_add(1);
            }
        }
        
        block
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
            // 定期访问内存
            thread::sleep(Duration::from_secs(MEMORY_KEEP_ALIVE_INTERVAL_SECS));
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
        

        // 定期检查是否需要调整内存
        thread::sleep(Duration::from_secs(MEMORY_CHECK_INTERVAL_SECS));
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
    
   let config: Config = match serde_yaml::from_str::<Config>(&config_content) {
    Ok(mut config) => {
        // 校验时间格式并存储解析结果
        config.work_start = parse_time(&config.work_start_time)
            .map_err(|e| format!("work_start_time {}", e))?;
        config.work_end = parse_time(&config.work_end_time)
            .map_err(|e| format!("work_end_time {}", e))?;

        clamp_percent(&mut config.work_cpu_usage, "work_cpu_usage");
        clamp_percent(&mut config.rest_cpu_usage, "rest_cpu_usage");
        clamp_percent(&mut config.work_memory_usage, "work_memory_usage");  // 添加这行
        clamp_percent(&mut config.rest_memory_usage, "rest_memory_usage");  // 添加这行

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
        let config_clone = Arc::clone(&shared_config);

        let handle = thread::spawn(move || {
            println!("启动 CPU 负载线程，核心 #{}", i);
            cpu_load(config_clone);
        });
        
        handles.push(handle);
    }
    
    println!("\n===== 所有线程已启动 =====");
    println!("程序运行中...");
    
    // 等待所有线程完成
    handles.push(memory_handle);
    for handle in handles {
          //handle.join().unwrap();
          if let Err(e) = handle.join() {
            eprintln!("线程意外退出: {:?}", e);
          }
        
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Local, TimeZone};

    fn make_config(
        work_start: &str,
        work_end: &str,
        work_days: Vec<String>,
        rest_days: Vec<String>,
    ) -> Config {
        Config {
            work_days,
            rest_days,
            work_start_time: work_start.to_string(),
            work_end_time: work_end.to_string(),
            work_cpu_usage: 70.0,
            rest_cpu_usage: 10.0,
            work_memory_usage: 70.0,
            rest_memory_usage: 20.0,
            work_start: parse_time(work_start).unwrap(),
            work_end: parse_time(work_end).unwrap(),
        }
    }

    #[test]
    fn percent_to_mb_negative() {
        let mm = MemoryManager::new();
        assert_eq!(mm.percent_to_mb(-10.0), 0);
    }

    #[test]
    fn percent_to_mb_above_hundred() {
        let mm = MemoryManager::new();
        let expected = mm.percent_to_mb(100.0);
        assert_eq!(mm.percent_to_mb(150.0), expected);
    }

    #[test]
    fn workday_detection() {
        let config = make_config("09:00", "18:00", vec![], vec![]);
        let date = NaiveDate::from_ymd_opt(2024, 10, 21).unwrap(); // Monday
        let dt_in = Local.from_local_datetime(&date.and_hms_opt(10, 0, 0).unwrap()).unwrap();
        assert!(is_work_time_at(&config, dt_in));

        let dt_out = Local.from_local_datetime(&date.and_hms_opt(20, 0, 0).unwrap()).unwrap();
        assert!(!is_work_time_at(&config, dt_out));
    }

    #[test]
    fn rest_day_detection() {
        let config = make_config(
            "09:00",
            "18:00",
            vec![],
            vec!["2024-10-21".to_string()],
        );
        let date = NaiveDate::from_ymd_opt(2024, 10, 21).unwrap();
        let dt = Local.from_local_datetime(&date.and_hms_opt(10, 0, 0).unwrap()).unwrap();
        assert!(!is_work_time_at(&config, dt));
    }

    #[test]
    fn cross_midnight_detection() {
        let config = make_config("22:00", "06:00", vec![], vec![]);

        // Monday 23:00 should be work time
        let mon = NaiveDate::from_ymd_opt(2024, 10, 21).unwrap();
        let dt_mon = Local.from_local_datetime(&mon.and_hms_opt(23, 0, 0).unwrap()).unwrap();
        assert!(is_work_time_at(&config, dt_mon));

        // Tuesday 05:00 should still be work time
        let tue = NaiveDate::from_ymd_opt(2024, 10, 22).unwrap();
        let dt_tue = Local.from_local_datetime(&tue.and_hms_opt(5, 0, 0).unwrap()).unwrap();
        assert!(is_work_time_at(&config, dt_tue));

        // Tuesday 07:00 should be rest time
        let dt_after = Local.from_local_datetime(&tue.and_hms_opt(7, 0, 0).unwrap()).unwrap();
        assert!(!is_work_time_at(&config, dt_after));
    }
}
