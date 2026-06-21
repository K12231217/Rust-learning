//! 应用逻辑层 —— 与硬件无关的状态机和策略。
//!
//! 关键约束:本文件**不依赖** `embassy_stm32` / `embassy_time` / 具体 GPIO 类型。
//! 它只回答「现在是什么状态、LED 该用多长周期闪」,不碰任何外设。
//!
//! 好处:这套逻辑可以在 PC 上做 host 单元测试(见文件末尾 `#[cfg(test)]`),
//! 不用每次都烧到板子上验证。这是从 `wg-core` 学到的「可测试 core」思想。

use crate::config;

/// 系统状态。LED 不只是「翻转输出」,而是系统状态指示器。
///
/// 对照 wg110-firmware 的 LED 状态语义(Booting/Standby/Connected/Error/Fatal),
/// 这里先做简化版三态,后续可平滑扩展。
#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SystemState {
    /// 启动中:快闪,提示正在初始化。
    Booting,
    /// 正常运行:1s 心跳。
    Running,
    /// 错误:250ms 快闪。
    ///
    /// 预留状态:错误处理路径尚未接入(后续步骤),先定义好语义和周期,
    /// 任何任务检测到不可恢复错误时 `runtime::publish_state(SystemState::Error)` 即可。
    #[allow(dead_code)]
    Error,
}

/// LED 闪烁控制器:纯逻辑,只根据状态算出「半周期(ms)」。
///
/// 它不知道 LED 接在哪个引脚,也不知道用 Ticker 还是 Timer——
/// 那些是 [`crate::tasks`] 的事。这里只做「状态 → 周期」的决策。
pub struct BlinkController {
    state: SystemState,
}

impl BlinkController {
    /// 以某个初始状态构造控制器。
    pub fn new(state: SystemState) -> Self {
        Self { state }
    }

    /// 当前状态。
    pub fn state(&self) -> SystemState {
        self.state
    }

    /// 切换到新状态。
    pub fn set_state(&mut self, state: SystemState) {
        self.state = state;
    }

    /// 当前状态下,LED 翻转的半周期(毫秒)。
    /// 周期参数全部来自 [`config`],本函数只做映射。
    pub fn half_period_ms(&self) -> u64 {
        match self.state {
            SystemState::Booting => config::LED_BOOTING_MS,
            SystemState::Running => config::LED_RUNNING_MS,
            SystemState::Error => config::LED_ERROR_MS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_state_to_period() {
        let mut c = BlinkController::new(SystemState::Booting);
        assert_eq!(c.half_period_ms(), config::LED_BOOTING_MS);

        c.set_state(SystemState::Running);
        assert_eq!(c.half_period_ms(), config::LED_RUNNING_MS);

        c.set_state(SystemState::Error);
        assert_eq!(c.half_period_ms(), config::LED_ERROR_MS);
    }
}
