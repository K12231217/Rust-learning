//! 任务层 —— 只做异步等待、收发事件、驱动硬件;决策交给 [`crate::app`]。

use crate::app::{BlinkController, ButtonEvent, SystemState};
use crate::board::{ButtonPin, LedPin};
use crate::{config, runtime};
use embassy_futures::select::{Either3, select3};
use embassy_time::{Duration, Ticker, Timer, with_timeout};

/// LED 任务:按状态闪烁,响应状态推送与按键手势。`ctrl` 是当前状态的唯一拥有者。
/// 用 `Ticker` 对齐绝对时刻,零累积漂移;换状态即重建 Ticker 换周期。
#[embassy_executor::task]
pub async fn blink_task(mut led: LedPin) {
    let mut ctrl = BlinkController::new(SystemState::Booting);
    let mut ticker = Ticker::every(Duration::from_millis(ctrl.half_period_ms()));
    defmt::info!(
        "[task:blink] started, state={} period={}ms",
        ctrl.state(),
        ctrl.half_period_ms()
    );

    loop {
        match select3(ticker.next(), runtime::wait_state(), runtime::wait_button_event()).await {
            // 翻转 LED —— 心跳本体,无日志。
            Either3::First(_) => led.toggle(),
            // 外部推送的新状态。
            Either3::Second(new_state) => {
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
            // 按键手势 → 映射到状态。
            Either3::Third(event) => {
                ctrl.set_state(event.to_state());
                ticker = Ticker::every(Duration::from_millis(ctrl.half_period_ms()));
                defmt::info!(
                    "[led] gesture={} -> state={} period={}ms",
                    event,
                    ctrl.state(),
                    ctrl.half_period_ms()
                );
            }
        }
    }
}

/// 按键手势状态机:把 PA0 边沿 + 时间识别成短按 / 双击 / 长按(低电平有效:按下=低)。
/// 只产出手势,不认识 LED / 状态;映射由 [`ButtonEvent::to_state`] 完成。
#[embassy_executor::task]
pub async fn button_task(mut button: ButtonPin) {
    let debounce = Duration::from_millis(config::BUTTON_DEBOUNCE_MS);
    let long_press = Duration::from_millis(config::BUTTON_LONG_PRESS_MS);
    let double_gap = Duration::from_millis(config::BUTTON_DOUBLE_GAP_MS);
    defmt::info!("[task:button] started (PA0, active-low)");

    loop {
        // 等按下并去抖。
        button.wait_for_falling_edge().await;
        Timer::after(debounce).await;
        if button.is_high() {
            continue; // 毛刺
        }

        // 长按阈值内是否松开 → 区分长按 / 短按。
        match with_timeout(long_press, button.wait_for_rising_edge()).await {
            // 超时仍按住 → 长按。
            Err(_) => {
                emit(ButtonEvent::LongPress);
                button.wait_for_rising_edge().await; // 等松开收尾
            }
            // 提前松开 → 看双击间隔内是否再次按下。
            Ok(_) => match with_timeout(double_gap, button.wait_for_falling_edge()).await {
                // 第二次按下 → 双击。
                Ok(_) => {
                    Timer::after(debounce).await;
                    emit(ButtonEvent::DoubleClick);
                    button.wait_for_rising_edge().await;
                }
                // 无第二次 → 短按。
                Err(_) => emit(ButtonEvent::ShortPress),
            },
        }
    }
}

/// 打日志 + 发手势到通道。
fn emit(event: ButtonEvent) {
    defmt::info!("[button] gesture detected: {}", event);
    runtime::send_button_event(event);
}
