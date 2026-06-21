//! 任务间通信通道:状态用 Signal(最新值即真相),手势用 Channel(不丢、按序)。

use crate::app::{ButtonEvent, SystemState};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;

/// 系统状态信号。多生产者写,LED 任务读。
static STATE: Signal<CriticalSectionRawMutex, SystemState> = Signal::new();

/// 按键手势通道。按键任务写,LED 任务按序消费。容量 4 缓冲连击。
static BUTTON: Channel<CriticalSectionRawMutex, ButtonEvent, 4> = Channel::new();

/// 发布新系统状态。
pub fn publish_state(state: SystemState) {
    defmt::info!("[runtime] state -> {}", state);
    STATE.signal(state);
}

/// 等待下一次状态变化。
pub async fn wait_state() -> SystemState {
    STATE.wait().await
}

/// 发布一个手势;通道满则丢弃并告警,不阻塞检测循环。
pub fn send_button_event(event: ButtonEvent) {
    if BUTTON.try_send(event).is_err() {
        defmt::warn!("[runtime] button channel full, dropping {}", event);
    }
}

/// 等待下一个手势。
pub async fn wait_button_event() -> ButtonEvent {
    BUTTON.receive().await
}
