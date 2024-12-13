use std::fs;
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};
use num_cpus;
use chrono::{Local, NaiveTime, Datelike, NaiveDate, Weekday};

#[derive(Debug, Serialize, Deserialize, Clone)]  // 添加 Clone
struct Config {
    work_days: Vec<String>,      // 指定工作日列表 格式："2025-01-01"
    rest_days: Vec<String>,      // 指定休息日列表 格式："2025-01-01"
    work_start_time: String,     // 工作开始时间 格式："09:00"
    work_end_time: String,       // 工作结束时间 格式："18:00"
    work_cpu_usage: f64,         // 工作时间 CPU 使用率
    rest_cpu_usage: f64,         // 休息时间 CPU 使用率
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
    let is_weekday = match now.weekday() {
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

fn main() {
    let config_content = fs::read_to_string("config.yml")
        .expect("无法读取配置文件");
    
    let config: Config = serde_yaml::from_str(&config_content)
        .expect("无法解析配置文件");
        
    println!("CPU 核心数: {}", num_cpus::get());
    println!("工作时间 CPU 使用率: {}%", config.work_cpu_usage);
    println!("休息时间 CPU 使用率: {}%", config.rest_cpu_usage);
    
    let mut handles = vec![];
    for i in 0..num_cpus::get() {
        let config_clone = config.clone();
        let handle = thread::spawn(move || {
            println!("Started CPU load thread for core {}", i);
            cpu_load(&config_clone);
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.join().unwrap();
    }
}
