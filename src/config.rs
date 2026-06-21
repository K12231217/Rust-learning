//! 配置 —— 固件身份 + 可调策略。硬件事实在 [`crate::board`]。

/// 固件版本号(取自 Cargo.toml)。
pub const FW_VERSION: &str = env!("CARGO_PKG_VERSION");

// ───── LED 闪烁半周期(ms)─────

/// 正常运行(心跳灯)。
pub const LED_RUNNING_MS: u64 = 1000;
/// 启动:快闪。
pub const LED_BOOTING_MS: u64 = 150;
/// 错误:快闪。
pub const LED_ERROR_MS: u64 = 250;

// ───── 按键手势识别阈值(ms)─────

/// 去抖:边沿后等这么久再确认电平。
pub const BUTTON_DEBOUNCE_MS: u64 = 20;
/// 长按阈值:按住超过即判长按。
pub const BUTTON_LONG_PRESS_MS: u64 = 800;
/// 双击间隔:松开后这么久内再按即判双击,否则短按。
pub const BUTTON_DOUBLE_GAP_MS: u64 = 300;
