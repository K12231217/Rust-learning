//! 应用任务层 —— Embassy async task。
//!
//! 职责边界(学自 wg110-firmware 的 task / core 分工):
//! - **只**负责:异步等待、收发事件([`crate::runtime`])、驱动硬件、打日志;
//! - **不**负责:决定「该闪多快」——那是 [`crate::app::BlinkController`] 的纯逻辑。
//!
//! 任务只依赖 BSP 暴露的类型别名(如 [`LedPin`]),不直接触碰寄存器,
//! 也不关心 LED 接在哪个引脚。换板子时本文件零修改。

use crate::app::{BlinkController, SystemState};
use crate::board::LedPin;
use crate::{config, runtime};
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Ticker};

/// LED 状态指示任务:按当前系统状态决定闪烁周期。
///
/// 用 `Ticker` 而非 `Timer::after` —— Ticker 按绝对时刻对齐,自动补偿循环体耗时,
/// **零累积漂移**。每次状态变化就重建 Ticker,立即换上新周期。
///
/// 这里用 `select` 同时等两件事:
/// 1. ticker 到点 → 翻转 LED;
/// 2. runtime 发来新状态 → 切换周期(任务间解耦,不被谁直接调用)。
#[embassy_executor::task]
pub async fn blink_task(mut led: LedPin) {
    // 初始状态:启动中(快闪)。真正的状态由 main / 其他任务通过 runtime 推送。
    let mut ctrl = BlinkController::new(SystemState::Booting);
    let mut ticker = Ticker::every(Duration::from_millis(ctrl.half_period_ms()));
    defmt::info!(
        "[task:blink] started, state={} period={}ms",
        ctrl.state(),
        ctrl.half_period_ms()
    );

    loop {
        match select(ticker.next(), runtime::wait_state()).await {
            // —— ticker 到点:翻转 LED ——
            Either::First(_) => {
                led.toggle();
            }
            // —— 收到新状态:更新控制器并立即换周期 ——
            Either::Second(new_state) => {
                if new_state != ctrl.state() {
                    ctrl.set_state(new_state);
                    ticker = Ticker::every(Duration::from_millis(ctrl.half_period_ms()));
                    defmt::info!(
                        "[led] state={} period={}ms",
                        ctrl.state(),
                        ctrl.half_period_ms()
                    );
                }
            }
        }
    }
}

/// 心跳日志:固定周期打点,不占用任何硬件。周期来自 [`config`]。
#[embassy_executor::task]
pub async fn heartbeat_task() {
    let mut ticker = Ticker::every(Duration::from_millis(config::HEARTBEAT_MS));
    let mut count = 0u32;
    defmt::info!("[task:heartbeat] started period={}ms", config::HEARTBEAT_MS);
    loop {
        ticker.next().await;
        count += 1;
        defmt::info!("[heartbeat] tick {}", count);
    }
}
