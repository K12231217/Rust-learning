#![no_std]
#![no_main]

mod board;
mod tasks;

use board::Board;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let board = Board::init();
    defmt::info!("== board ready, sysclk should be 96 MHz ==");

    // 把板级资源分发给各任务,启动并发。main 自身随后返回,任务持续运行。
    spawner.spawn(tasks::blink_task(board.led).unwrap());
    spawner.spawn(tasks::heartbeat_task().unwrap());
}
