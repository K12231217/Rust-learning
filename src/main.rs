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
    // ───── Boot 阶段:初始化时钟 + GPIO ─────
    defmt::info!(
        "[boot] {} firmware v{}",
        board::BOARD_NAME,
        config::FW_VERSION
    );
    defmt::info!("[boot] target {}", board::TARGET_CHIP);
    let board = Board::init();
    defmt::info!(
        "[boot] board ready, sysclk={}Hz hse={}Hz",
        board::SYSCLK_HZ,
        board::HSE_HZ
    );

    // ───── Start 阶段:启动任务 ─────
    // LED 任务初始进入 Booting(快闪),表示「还在启动」。
    spawner.spawn(tasks::blink_task(board.led).unwrap());
    spawner.spawn(tasks::heartbeat_task().unwrap());

    // ───── Run 阶段:启动完成,通过 runtime 把状态切到 Running ─────
    // 演示任务间解耦通信:main 不直接碰 LED,只发布状态,blink_task 自行响应。
    Timer::after(Duration::from_millis(600)).await;
    runtime::publish_state(SystemState::Running);
    defmt::info!("[boot] startup complete, entering run phase");
}
