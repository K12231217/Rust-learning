//! 板级支持包 (BSP) —— 本工程对「这块板子」的唯一硬件描述来源。
//!
//! 设计原则:所有与具体硬件绑定的东西(晶振频率、引脚分配、外设初始化)
//! 都收敛在这里。换板子或板子改版,只改本文件,main.rs 与 tasks.rs 不动。
//!
//! 对照 CubeMX:本文件 ≈ SystemClock_Config + MX_GPIO_Init + ... 的总和,
//! 但不是生成的死代码,而是返回一组「已初始化、类型安全」的资源句柄。

use embassy_stm32::Config;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::rcc::{
    AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllMul, PllPDiv, PllPreDiv, PllQDiv, PllSource,
    Sysclk,
};
use embassy_stm32::time::Hertz;

/// 目标芯片型号(人类可读,仅用于日志/诊断)。
pub const TARGET_CHIP: &str = "STM32F411CE";

/// 板型名称(人类可读,仅用于日志/诊断)。
pub const BOARD_NAME: &str = "WeAct BlackPill";

/// 板载晶振频率(Hz)。WeAct STM32F411「黑药丸」= 25 MHz。
/// ⚠️ 换板子先核对板上晶振丝印!填错会导致所有时序/波特率出错,甚至开机卡死。
/// 改这里通常要同步改下面 [`Board::clock_config`] 的 PLL 分频——它们是一次逻辑改动。
pub const HSE_HZ: u32 = 25_000_000;

/// 目标系统时钟(Hz),仅用于日志展示;真正的分频在 [`Board::clock_config`] 里。
pub const SYSCLK_HZ: u32 = 96_000_000;

/// 板载晶振频率,包成 HAL 需要的 [`Hertz`] 类型。
const HSE_FREQ: Hertz = Hertz(HSE_HZ);

/// 板载用户 LED 的类型别名。
/// 对外只暴露这个名字,应用层不需要知道它实际接在 PC13。
pub type LedPin = Output<'static>;

/// 初始化完成、可直接使用的板级资源集合。
/// 相当于 CubeMX 跑完所有 MX_*_Init 后交到你手里的一组外设句柄。
pub struct Board {
    /// 板载用户 LED(PC13,低电平点亮)。
    pub led: LedPin,
}

impl Board {
    /// 上电初始化:配置时钟树 + 初始化所有板载外设,返回就绪的 [`Board`]。
    /// 等价于 CubeMX 的 HAL_Init + SystemClock_Config + MX_GPIO_Init 之和。
    pub fn init() -> Self {
        // 先配时钟,再交给 HAL 初始化。init() 返回该芯片全部外设的所有权令牌 p。
        let p = embassy_stm32::init(Self::clock_config());

        // —— 板载 LED:PC13,初始低电平(上电即亮),低翻转速度即可 ——
        let led = Output::new(p.PC13, Level::Low, Speed::Low);

        // 此处可继续初始化更多板载外设(串口、按键、传感器...),
        // 加进 Board 结构体一并返回。目前只有 LED。

        Board { led }
    }

    /// 时钟树配置(板级专属——因为它依赖板载晶振)。
    /// 对应 CubeMX 的 Clock Configuration 页。目标:SYSCLK 96MHz,USB 时钟 48MHz。
    ///
    /// PLL 公式:VCO_in = HSE/PLLM(需 1~2MHz)→ VCO_out = VCO_in×PLLN → SYSCLK = VCO_out/PLLP
    fn clock_config() -> Config {
        let mut config = Config::default();

        config.rcc.hse = Some(Hse {
            freq: HSE_FREQ,
            mode: HseMode::Oscillator, // 用晶振(而非外部时钟输入)
        });
        config.rcc.pll_src = PllSource::HSE;
        config.rcc.pll = Some(Pll {
            prediv: PllPreDiv::DIV25,  // 25MHz / 25 = 1MHz   (VCO 输入)
            mul: PllMul::MUL192,       // 1MHz × 192 = 192MHz (VCO 输出)
            divp: Some(PllPDiv::DIV2), // 192MHz / 2 = 96MHz  → SYSCLK
            divq: Some(PllQDiv::DIV4), // 192MHz / 4 = 48MHz  → USB/SDIO
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P; // 系统时钟选 PLL 的 P 输出
        config.rcc.ahb_pre = AHBPrescaler::DIV1; // HCLK 96MHz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // APB1 48MHz(F411 限制 ≤50MHz)
        config.rcc.apb2_pre = APBPrescaler::DIV1; // APB2 96MHz

        config
    }
}
