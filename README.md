# CPU Load Simulator

This project is a CPU load simulator that adjusts the CPU usage based on the configured working hours and rest hours. It can be used to test and simulate system performance under different CPU load conditions.

## Features

- Adjusts CPU usage based on configured working hours and rest hours.
- Supports custom workdays and rest days.
- Multi-threaded CPU load simulation, supporting multi-core CPUs.
- Simulates memory usage during work and rest periods using `work_memory_usage` and `rest_memory_usage`.

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/kakahu2015/cpu.git
   cd cpu
   ```

## Build the executable：
   cargo build --release

## Configuration
Create a config.yml file in the root directory of the project with the following format:
work_days: ["2024-12-24"]         # List of specific workdays in the format "YYYY-MM-DD"
rest_days: ["2024-12-25"]         # List of specific rest days in the format "YYYY-MM-DD"
work_start_time: "09:00"          # Work start time in the format "HH:MM"
work_end_time: "18:00"            # Work end time in the format "HH:MM"
work_cpu_usage: 50.0              # CPU usage during work hours (%)
rest_cpu_usage: 10.0              # CPU usage during rest hours (%)

## Usage
Run the following command to start the program:
cargo run --release

The program will read the configuration from the config.yml file and adjust the CPU and memory usage based on the current time. Each CPU core will have a thread to simulate the load.
`work_memory_usage` specifies the desired memory usage during work hours, while `rest_memory_usage` sets the memory usage for rest periods. Configure them in `config.yml` as shown below:


## Example
work_days: ["2024-12-24"]
rest_days: ["2024-12-25"]
work_start_time: "09:00"
work_end_time: "18:00"
work_cpu_usage: 50.0
rest_cpu_usage: 10.0
work_memory_usage: 60.0
rest_memory_usage: 20.0

On December 24, 2024, between 09:00 and 18:00, the CPU usage will be set to 50% and memory usage 60%. During other times and on December 25, the CPU usage will be 10% and memory usage 20%.

