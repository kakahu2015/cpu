use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use chrono::{NaiveDateTime, Datelike, Timelike, NaiveTime, Local};
use serde::{Deserialize, Serialize};
use num_cpus;
use log::{info, warn, error};
use ctrlc;

#[derive(Debug, Serialize, Deserialize)]
struct TimeRange {
    start: String,  // Format: "HH:MM"
    end: String,    // Format: "HH:MM"
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkloadConfig {
    cpu_usage: f64,
    calculation_intensity: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    workday_load: WorkloadConfig,
    non_workday_load: WorkloadConfig,
    after_hours_load: WorkloadConfig,
    work_hours: TimeRange,
    holidays: Vec<String>,  // Format: "YYYY-MM-DD"
    cycle_duration_ms: u64,
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        for load in [&self.workday_load, &self.non_workday_load, &self.after_hours_load].iter() {
            if load.cpu_usage <= 0.0 || load.cpu_usage > 100.0 {
                return Err("CPU 使用率必须在 0-100 之间".to_string());
            }
        }
        
        // 验证时间格式
        self.parse_time(&self.work_hours.start)?;
        self.parse_time(&self.work_hours.end)?;
        
        // 验证假期日期格式
        for date in &self.holidays {
            if NaiveDateTime::parse_from_str(&format!("{} 00:00:00", date), "%Y-%m-%d %H:%M:%S").is_err() {
                return Err(format!("无效的日期格式: {}", date));
            }
        }
        
        if self.cycle_duration_ms == 0 {
            return Err("周期时间必须大于0".to_string());
        }
        
        Ok(())
    }
    
    fn parse_time(&self, time_str: &str) -> Result<NaiveTime, String> {
        NaiveTime::parse_from_str(time_str, "%H:%M")
            .map_err(|e| format!("无效的时间格式 {}: {}", time_str, e))
    }
    
    fn get_current_workload(&self) -> &WorkloadConfig {
        let now = Local::now();
        let current_date = now.format("%Y-%m-%d").to_string();
        let current_time = now.time();
        
        // 检查是否是假期
        if self.holidays.contains(&current_date) {
            return &self.non_workday_load;
        }
        
        // 检查是否是周末
        if now.weekday().number_from_monday() > 5 {
            return &self.non_workday_load;
        }
        
        // 检查是否在工作时间
        let work_start = self.parse_time(&self.work_hours.start).unwrap();
        let work_end = self.parse_time(&self.work_hours.end).unwrap();
        
        if current_time >= work_start && current_time < work_end {
            &self.workday_load
        } else {
            &self.after_hours_load
        }
    }
}

fn cpu_load(
    config: Arc<Config>,
    running: Arc<AtomicBool>,
    core_id: usize,
) {
    info!("Started CPU load thread for core {}", core_id);
    
    while running.load(Ordering::Relaxed) {
        let workload = config.get_current_workload();
        let cycle_duration = Duration::from_millis(config.cycle_duration_ms);
        
        let work_ratio = workload.cpu_usage / 100.0;
        let work_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * work_ratio);
        let sleep_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * (1.0 - work_ratio));
        
        let start = Instant::now();
        
        // 执行负载
        while start.elapsed() < work_duration {
            let mut x: f64 = 0.0;
            for _ in 0..workload.calculation_intensity {
                x += 1.0;
                x = x.sqrt();
            }
        }
        
        if start.elapsed() > work_duration {
            warn!(
                "Core {} exceeded work duration: {:?}",
                core_id,
                start.elapsed()
            );
        }
        
        thread::sleep(sleep_duration);
    }
    
    info!("Stopping CPU load thread for core {}", core_id);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    env_logger::init();
    
    // 读取配置
    let config: Config = serde_yaml::from_str(
        &fs::read_to_string("config.yml")
            .map_err(|e| format!("无法读取配置文件: {}", e))?
    )?;
    
    // 验证配置
    config.validate()?;
    
    let config = Arc::new(config);
    
    info!(
        "Starting scheduled CPU loader with {} cores",
        num_cpus::get()
    );
    
    // 退出信号控制
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        info!("Received shutdown signal");
        r.store(false, Ordering::Relaxed);
    })?;
    
    // 创建工作线程
    let mut handles = vec![];
    
    for i in 0..num_cpus::get() {
        let running = running.clone();
        let config = config.clone();
        
        let handle = thread::spawn(move || {
            cpu_load(config, running, i);
        });
        
        handles.push(handle);
    }
    
    // 等待所有线程完成
    for handle in handles {
        handle.join().unwrap();
    }
    
    info!("All threads terminated successfully");
    Ok(())
}
