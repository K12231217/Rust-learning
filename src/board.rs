//! BSP —— 这块板子的唯一硬件描述来源。换板/改版只动本文件。

use embassy_stm32::Config;
use embassy_stm32::bind_interrupts;
use embassy_stm32::exti::{ExtiInput, InterruptHandler};
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Async;
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource,
    Sysclk,
};
use embassy_stm32::time::Hertz;

// PA0 走 EXTI 通道 0;异步等边沿必须绑定其中断。
bind_interrupts!(struct Irqs {
    EXTI0 => InterruptHandler<embassy_stm32::interrupt::typelevel::EXTI0>;
});

/// 芯片型号(仅日志用)。
pub const TARGET_CHIP: &str = "STM32F411CE";

/// 板型名(仅日志用)。
pub const BOARD_NAME: &str = "WeAct BlackPill";

/// 板载晶振频率。⚠️ 换板先核对丝印;改这里要同步改 [`Board::clock_config`] 的 PLL 分频。
pub const HSE_HZ: u32 = 25_000_000;

/// 目标系统时钟(仅日志用;实际分频见 [`Board::clock_config`])。
pub const SYSCLK_HZ: u32 = 96_000_000;

const HSE_FREQ: Hertz = Hertz(HSE_HZ);

/// 板载 LED(实际接 PC13)。
pub type LedPin = Output<'static>;

/// 板载按键(实际接 PA0,支持异步等边沿)。
pub type ButtonPin = ExtiInput<'static, Async>;

/// 初始化完成、可直接使用的板级资源。
pub struct Board {
    /// 用户 LED(PC13,低电平点亮)。
    pub led: LedPin,
    /// 用户按键 KEY(PA0,低电平有效,需内部上拉)。
    pub button: ButtonPin,
}

impl Board {
    /// 上电初始化:配时钟 + 初始化外设。
    pub fn init() -> Self {
        let p = embassy_stm32::init(Self::clock_config());

        // LED:PC13,初始低电平(上电即亮)。
        let led = Output::new(p.PC13, Level::Low, Speed::Low);
        // 按键:PA0,内部上拉,低电平有效。
        let button = ExtiInput::new(p.PA0, p.EXTI0, Pull::Up, Irqs);

        Board { led, button }
    }

    /// 时钟树:SYSCLK 96MHz,USB 48MHz。
    /// PLL:VCO_in = HSE/PLLM(1~2MHz)→ VCO_out = VCO_in×PLLN → SYSCLK = VCO_out/PLLP
    fn clock_config() -> Config {
        let mut config = Config::default();

        config.rcc.hse = Some(Hse {
            freq: HSE_FREQ,
            mode: HseMode::Oscillator,
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV25,  // 25MHz / 25 = 1MHz
            mul: PllMul::MUL192,       // 1MHz × 192 = 192MHz
            divp: Some(PllPDiv::DIV2), // 192MHz / 2 = 96MHz → SYSCLK
            divq: Some(PllQDiv::DIV4), // 192MHz / 4 = 48MHz → USB
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1; // HCLK 96MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // APB1 48MHz(≤50MHz)
        config.rcc.apb2_pre = APBPrescaler::DIV1; // APB2 96MHz

        config
    }
}
