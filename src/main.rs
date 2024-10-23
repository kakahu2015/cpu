use std::fs;
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use num_cpus; // 新添加依赖，用于获取 CPU 核心数

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    cpu_usage: f64,
}

fn cpu_load(work_duration: Duration, sleep_duration: Duration) {
    loop {
        let start = Instant::now();
        
        // 进行计算消耗 CPU
        while start.elapsed() < work_duration {
            let mut x: f64 = 0.0;
            for _ in 0..1000 {
                x += 1.0;
                x = x.sqrt();
            }
        }

        // 休眠以达到目标使用率
        thread::sleep(sleep_duration);
    }
}

fn main() {
    // 读取配置文件
    let config_content = fs::read_to_string("config.yml")
        .expect("无法读取配置文件");
    
    let config: Config = serde_yaml::from_str(&config_content)
        .expect("无法解析配置文件");

    println!("目标 CPU 使用率: {}%", config.cpu_usage);
    println!("CPU 核心数: {}", num_cpus::get());

    // 计算工作时间和休眠时间的比例
    let work_ratio = config.cpu_usage / 100.0;
    let sleep_ratio = 1.0 - work_ratio;

    // 基准周期时间（毫秒）
    let cycle_duration = Duration::from_millis(100);
    let work_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * work_ratio);
    let sleep_duration = Duration::from_secs_f64(cycle_duration.as_secs_f64() * sleep_ratio);

    // 创建与 CPU 核心数量相同的线程
    let mut handles = vec![];
    
    for i in 0..num_cpus::get() {
        let work_dur = work_duration;
        let sleep_dur = sleep_duration;
        
        let handle = thread::spawn(move || {
            println!("Started CPU load thread for core {}", i);
            cpu_load(work_dur, sleep_dur);
        });
        
        handles.push(handle);
    }

    // 等待所有线程完成（实际上是永远循环的）
    for handle in handles {
        handle.join().unwrap();
    }
}
