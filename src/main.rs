use std::fs;
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use openssl::version;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    cpu_usage: f64,
}

fn main() {
    // 打印 OpenSSL 版本信息，验证链接成功
    println!("OpenSSL version: {}", version::version());
    
    // 读取配置文件
    let config_content = fs::read_to_string("config.yml")
        .expect("无法读取配置文件");
    
    let config: Config = serde_yaml::from_str(&config_content)
        .expect("无法解析配置文件");

    println!("目标 CPU 使用率: {}%", config.cpu_usage);

    // 计算工作时间和休眠时间的比例
    let work_ratio = config.cpu_usage / 100.0;
    let sleep_ratio = 1.0 - work_ratio;

    // 基准周期时间（毫秒）
    let cycle_duration = Duration::from_millis(100);
    let work_duration = Duration::from_float_secs(cycle_duration.as_secs_f64() * work_ratio);
    let sleep_duration = Duration::from_float_secs(cycle_duration.as_secs_f64() * sleep_ratio);

    loop {
        let start = Instant::now();
        
        // 进行计算消耗 CPU
        while start.elapsed() < work_duration {
            // 执行一些计算来消耗 CPU
            let mut x = 0.0;
            for _ in 0..1000 {
                x += 1.0;
                x = x.sqrt();
            }
        }

        // 休眠以达到目标使用率
        thread::sleep(sleep_duration);
    }
}
