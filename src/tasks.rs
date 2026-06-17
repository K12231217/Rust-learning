//! 应用任务层 —— 业务逻辑。
//!
//! 这里的任务只依赖 BSP 暴露的类型别名(如 [`LedPin`]),不直接触碰寄存器、
//! 也不关心 LED 接在哪个引脚。换板子时本文件零修改。

use crate::board::LedPin;
use embassy_time::{Duration, Ticker};

/// 心跳灯:固定 1s 周期翻转 LED。
///
/// 用 `Ticker` 而非 `Timer::after` —— Ticker 按绝对时刻对齐,
/// 自动补偿循环体的执行耗时,**零累积漂移**(对照之前日志里 after_millis 的漂移)。
#[embassy_executor::task]
pub async fn blink_task(mut led: LedPin) {
    let mut ticker = Ticker::every(Duration::from_millis(1000));
    loop {
        ticker.next().await;
        led.toggle();
        defmt::info!("[blink] toggled LED");
    }
}

/// 心跳日志:固定 300ms 周期打点,不占用任何硬件。
#[embassy_executor::task]
pub async fn heartbeat_task() {
    let mut ticker = Ticker::every(Duration::from_millis(300));
    let mut count = 0u32;
    loop {
        ticker.next().await;
        count += 1;
        defmt::info!("[heartbeat] tick {}", count);
    }
}
