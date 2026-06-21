//! 轻量运行时 —— 任务间通信的数据通道。
//!
//! 设计思想(学自 wg110-firmware 的全局 `RUNTIME`,但极度简化):
//! > 任务之间不要互相直接调用,而应该通过清晰的数据通道解耦。
//!
//! 这里用一个全局 [`Signal`] 发布「系统状态」。谁想改变 LED 行为,
//! 就 `publish_state(...)`;LED 任务 `wait` 这个信号,自己决定怎么响应。
//! 发布者和消费者互不认识,后续加按键/串口任务时不会形成网状耦合。
//!
//! 为什么用 Signal 而非 Channel:状态是「最新值即真相」的语义,
//! 只关心最后一次状态,不需要排队历史事件——这正是 Signal 的设计场景。

use crate::app::SystemState;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

/// 全局系统状态信号。多个生产者可写,LED 任务作为消费者读。
static STATE: Signal<CriticalSectionRawMutex, SystemState> = Signal::new();

/// 发布一个新的系统状态(任意任务都可调用)。
pub fn publish_state(state: SystemState) {
    defmt::info!("[runtime] state -> {}", state);
    STATE.signal(state);
}

/// 等待下一次状态变化(LED 任务用)。
pub async fn wait_state() -> SystemState {
    STATE.wait().await
}
