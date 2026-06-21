//! 应用逻辑层 —— 与硬件无关的纯状态机,可 host 单测。

use crate::config;

/// 系统状态(LED 即状态指示器)。
#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum SystemState {
    /// 启动中:快闪。
    Booting,
    /// 正常运行:1s 心跳。
    Running,
    /// 错误:250ms 快闪。
    Error,
}

/// 按键手势(按键状态机识别出的语义事件,非裸边沿)。
#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ButtonEvent {
    /// 短按:快按快松,后续无第二次按下。
    ShortPress,
    /// 双击:短按后在间隔阈值内再按一次。
    DoubleClick,
    /// 长按:按住超过长按阈值。
    LongPress,
}

impl ButtonEvent {
    /// 手势 → 状态(演示映射,可按需改)。
    pub fn to_state(self) -> SystemState {
        match self {
            ButtonEvent::ShortPress => SystemState::Running,
            ButtonEvent::DoubleClick => SystemState::Error,
            ButtonEvent::LongPress => SystemState::Booting,
        }
    }
}

/// LED 闪烁控制器:只根据状态算半周期(ms)。
pub struct BlinkController {
    state: SystemState,
}

impl BlinkController {
    pub fn new(state: SystemState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> SystemState {
        self.state
    }

    pub fn set_state(&mut self, state: SystemState) {
        self.state = state;
    }

    /// 当前状态对应的 LED 翻转半周期(ms),取自 [`config`]。
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

    #[test]
    fn gesture_maps_to_distinct_state() {
        assert_eq!(ButtonEvent::ShortPress.to_state(), SystemState::Running);
        assert_eq!(ButtonEvent::DoubleClick.to_state(), SystemState::Error);
        assert_eq!(ButtonEvent::LongPress.to_state(), SystemState::Booting);
    }
}
