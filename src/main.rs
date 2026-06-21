#![no_std]
#![no_main]

mod app;
mod board;
mod config;
mod runtime;
mod tasks;

use app::SystemState;
use board::Board;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Boot:初始化时钟 + GPIO。
    defmt::info!("[boot] {} firmware v{}", board::BOARD_NAME, config::FW_VERSION);
    defmt::info!("[boot] target {}", board::TARGET_CHIP);
    let board = Board::init();
    defmt::info!(
        "[boot] board ready, sysclk={}Hz hse={}Hz",
        board::SYSCLK_HZ,
        board::HSE_HZ
    );

    // Start:启动任务(心跳即 LED 在 Running 态慢闪,无单独 heartbeat 日志)。
    spawner.spawn(tasks::blink_task(board.led).unwrap());
    spawner.spawn(tasks::button_task(board.button).unwrap());

    // Run:600ms 后切到 Running(经 runtime 解耦,不直接碰 LED)。
    Timer::after(Duration::from_millis(600)).await;
    runtime::publish_state(SystemState::Running);
    defmt::info!("[boot] startup complete, entering run phase");
}
