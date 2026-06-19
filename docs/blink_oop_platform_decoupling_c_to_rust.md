# blink：把 C 的「面向对象 + 平台解耦」迁移到 Rust

> 视角：具有量产经验的嵌入式架构师
> 对象：当前 `blink`（STM32F411 + Embassy）项目
> 参照：`qiyuan_lx` 的 C 版 LED BSP（`feature/led-bridge` 分支）
> 姊妹篇：`blink_architecture_learning_from_wg110.md`

---

## 0. 这份文档解决什么问题

你在 C 里已经实现了一套相当成熟的「面向对象 + 平台解耦」LED BSP，希望把这套思路迁移到 Rust 项目，并作为未来「新增很多外设、分模块可复用」的样板。

本文回答一个问题：

> **这套 C 设计，哪些该带到 Rust，哪些不该？**

一句话结论先放这里：

> **借思想，扔机制。** 你 C 里那一千多行样板，大半是在用 C 手工「模拟」Rust + Embassy 语言原生就有的东西。设计意图（依赖倒置、平台解耦、消息解耦、可测试）全部值得带走；实现机制（函数指针 vtable、注册表、`is_inited` 哨兵、轮询 worker、`#ifdef` 分叉）在 Rust 里都有更安全的原生替代，不要照搬——**照搬等于用 Rust 写 C，既丢了编译期安全，也丢了 Rust 的工程价值。**

---

## 1. 源参照：qiyuan_lx 的 C LED BSP 在做什么

它的核心思想（作者自述）：**用 C 模拟 C++ 的对象封装，借鉴 Linux 的 ops 表 + 注册机制**，把 LED 拆成两层：

| 层 | 文件 | 职责 |
|---|---|---|
| **driver** | `Bsp/led/driver` | 单个 LED 对象：状态 + 闪烁算法，通过**注入的接口**驱动硬件，**不知道引脚** |
| **handler** | `Bsp/led/handler` | 多实例管理器 + 注册中心 + RTOS worker/queue，按 index 调度闪烁命令 |
| **system_adaption** | `System/system_adaption.c` | **唯一**知道 STM32 HAL + FreeRTOS 的文件，把具体实现注入抽象接口 |
| **app** | 调用方 | 只通过 handler 下命令，不碰硬件 |

它的关键手法是**函数指针结构体注入**（依赖倒置的 C 写法）：

```c
typedef struct {
    led_status_t (*pf_led_on) (void);   // 抽象的「开灯」
    led_status_t (*pf_led_off)(void);   // 抽象的「关灯」
} led_operations_t;                     // ← 一张 ops 表 = 一个接口
```

driver 只持有 `led_operations_t*`，**它不知道 LED 接在哪个引脚**；具体的 `HAL_GPIO_WritePin` 由 `system_adaption.c` 提供并注入。这就是「面向接口编程 / 平台解耦」。

**这个意图是完全正确的，也正是值得迁移的东西。** 下面看怎么迁移。

---

## 2. 值得借鉴的设计思想（思想层，直接带走）

这几条恰好和 `blink` 之前定下的方向**严丝合缝**——你的 C 工程自己印证了方向：

| C 里的设计 | Rust 里的对应 | 说明 |
|---|---|---|
| `system_adaption.c` 是唯一 include HAL/RTOS 的文件 | **`board.rs` 是唯一出现 `embassy_stm32` 具体类型的地方** | 思想 100% 一致，直接迁移 |
| `led_operations_t` 函数指针表（面向接口） | **驱动泛型于 trait**（首选 `embedded-hal` 的 `OutputPin`） | 同样是「依赖抽象不依赖实现」，但零开销、零手写 |
| app 发事件、不阻塞等闪完（queue 解耦） | **任务间用 channel / signal 解耦** | `embassy_sync::Channel` |
| 三级测试（纯 driver / 集成 / 系统） | **能脱硬件的逻辑放可测 core** | 纯逻辑用 host `cargo test` + `embedded-hal-mock` |
| driver / handler / adaption 三层职责 | 分层（但 handler 那层大半被 Embassy runtime 吃掉，见 §5） | 职责边界思想保留 |

**核心原则（语言无关，永远值得守）：**

> 硬件、业务、调度、平台适配各自有清晰边界；上层依赖抽象，不依赖具体引脚 / 具体 OS。

---

## 3. 该用 Rust 原生替代的机制（机制层，别照搬）

这是本文最值钱的对照表。你 C 里写的每一坨样板，几乎都对应 Rust 的一个语言特性——它们是在补 C 的坑，而 Rust 没有这些坑：

| C 的机制 | 它在补什么坑 | Rust / Embassy 原生替代 |
|---|---|---|
| 函数指针 vtable（`pf_led_on`…） | C 没有 trait | **trait + 泛型**，编译期静态分发，零开销，零手写 |
| 注入 `time_base_ms_t`（`HAL_GetTick`） | C 没有标准时基 | `embassy_time::{Instant, Timer}`，**不用注入** |
| 注入 `os_delay_t` / `os_thread` / `os_queue` / `os_critical` | C 没有标准并发原语 | Embassy 本身就是运行时：`#[task]` spawn、`embassy_sync::Channel`、`Mutex`，**不用注入** |
| `is_inited` 标志 + `INIT_PATTERN(0xA6A6A6A6)` 哨兵 | C 没有 `Option` | `Option<T>` / 所有权——「拿到对象 = 已就绪」 |
| 构造失败手动把接口指针置 NULL 回滚 | C 没有 RAII / Result | `fn new() -> Result<Self, E>`：要么有效，要么没有，**不存在半初始化** |
| `#ifdef OS_SUPPORTING` 维护两套世界 | C 只能靠预处理器做可选 | cargo features / 泛型，无预处理器 |
| 1Hz 轮询 worker（`vTaskDelay(1000)` 查队列） | C 裸机思维 | `channel.receive().await`，**零延迟、零轮询** |
| 手写注册数组 + 计数器 + 两个容量常量 | C 没有标准容器 | `[Option<_>; N]` / `heapless::Vec`，边界由类型系统保证 |
| driver 和 handler 里**抄了两份**的 blink 算法 | 复制粘贴 | 一个 `async fn blink()` 复用 |

**meta 结论：** C 用大量样板「模拟」对象与并发；Rust 里这些是语言原生的。所以迁移时，**接口表→trait、注入的 OS→Embassy 原生、哨兵/`is_inited`→`Option`/所有权、轮询 worker→`await` channel**。

---

## 4. 落到代码：C 的一坨 → Rust 十几行

### 4.1 驱动层：`bsp_led_driver` → 泛型驱动

C 里需要：`led_operations_t` ops 表 + `led_driver_inst`（注入+校验+回滚）+ `is_inited` + `led_blink`（每毫秒重复 set 的阻塞双循环）。Rust 里坍缩成：

```rust
// drivers/led.rs —— 可复用、平台无关、可 host 测试
// 没有 ops 表、没有 inst、没有 is_inited、没有 INIT_PATTERN
use embedded_hal::digital::OutputPin;
use embassy_time::{Duration, Timer};

#[derive(Clone, Copy)]
pub enum Proportion { OneToOne, OneToTwo, OneToThree } // 亮占比 1/2、1/3、1/4（对应 C 的 PROPORTIONN_*）

impl Proportion {
    const fn on_div(self) -> u32 {
        match self { Self::OneToOne => 2, Self::OneToTwo => 3, Self::OneToThree => 4 }
    }
}

/// 泛型于「能输出电平的东西」= 你的 led_operations_t
pub struct Led<P> { pin: P }

impl<P: OutputPin> Led<P> {
    pub fn new(pin: P) -> Self { Self { pin } } // 不可能半初始化

    pub async fn blink(&mut self, cycle: Duration, times: u32, prop: Proportion) {
        let on = cycle / prop.on_div();
        let off = cycle - on;
        for _ in 0..times {
            let _ = self.pin.set_high();
            Timer::after(on).await;   // 不阻塞别的任务、可被取消（C 的阻塞 worker 做不到）
            let _ = self.pin.set_low();
            Timer::after(off).await;
        }
    }
}
```

> **极性（高低有效）属于 board，不属于 driver。** 你的 PC13 是低电平点亮。保持 driver 极性无关的干净做法：在 `board.rs` 里用一个反相封装的 newtype（`set_high` 内部调 `inner.set_low()`，并实现 `OutputPin`）交给 driver。想图省事也可以给 `Led::new` 加一个 `active_low: bool`，但那会让 driver 知道极性，轻微违反「driver 不知道引脚细节」原则。

### 4.2 平台适配层：`system_adaption.c` → `board.rs`

C 里 `system_adaption.c` 是唯一把 `HAL_GPIO_WritePin` 注入 ops 表的地方。Rust 里这个角色就是 `board.rs`——**唯一允许出现 `embassy_stm32` 具体类型**的文件：

```rust
// board.rs —— 唯一的平台适配层（= 你的 system_adaption.c）
pub type LedPin = Output<'static>;     // 具体引脚类型只在这里出现

pub struct Board { pub led: LedPin }

impl Board {
    pub fn init() -> Self {
        let p = embassy_stm32::init(Self::clock_config());
        let led = Output::new(p.PC13, Level::High, Speed::Low); // 这块板的事实：PC13、低电平亮
        Board { led }
    }
}
```

driver（`Led<P>`）泛型、可复用；board 提供具体 `LedPin` 注入进去。**换板只改 board.rs，driver 一行不动**——这正是你 C 里「换板只改 system_adaption」的同一条边界。

### 4.3 管理/调度层：`bsp_led_handler` → Embassy task + channel

这是迁移中**收益最大**的一层：你 C 里的 handler（注册表 + worker 线程 + queue + 临界区）**几乎整层蒸发**——Embassy 的 executor 就是你的 worker，`Channel` 就是你的 queue：

```rust
// 命令解耦：app 发命令，led_task 消费
// 没有 INIT_PATTERN、没有 1Hz 轮询、没有 const index 漏传 bug
use embassy_sync::channel::Receiver;

pub struct BlinkCmd { pub cycle: Duration, pub times: u32, pub prop: Proportion }

#[embassy_executor::task]
async fn led_task(
    mut led: Led<board::LedPin>,                 // 任务绑定本板的具体类型
    rx: Receiver<'static, RawMutex, BlinkCmd, 4>,
) {
    loop {
        let cmd = rx.receive().await;            // 零延迟，不是每秒轮询一次
        led.blink(cmd.cycle, cmd.times, cmd.prop).await;
    }
}
```

**分层要点：**

- `Led<P>` **泛型** = 可复用 + 可 host 测；
- `led_task` 用 `board::LedPin` **具体类型** = 这块板的胶水；
- 多个 LED → 多个独立 task，天然并发，不再是 C 里那个被阻塞串行化的单 worker。

### 4.4 可测 core：三级测试 → host `cargo test`

你 C 里 `Test_1`（纯 driver）/ `Test_2`（集成）/ `Test_3`（系统）的三级测试纪律非常对。Rust 里因为 driver 泛型于 trait，可以喂 `embedded-hal-mock` 在 **PC 上**直接测，无需上板：

```rust
// 用 mock pin 验证 blink 的电平时序，cargo test 在 PC 跑
#[tokio::test]
async fn blink_drives_pin() {
    use embedded_hal_mock::eh1::digital::{Mock, Transaction, State};
    let pin = Mock::new(&[
        Transaction::set(State::High), Transaction::set(State::Low),
        Transaction::set(State::High), Transaction::set(State::Low),
    ]);
    let mut led = Led::new(pin);
    led.blink(Duration::from_millis(10), 2, Proportion::OneToOne).await;
    // led.into_inner().done(); // 视封装而定，校验所有期望都被消费
}
```

> 把「与硬件无关的逻辑」留在泛型 driver / 纯状态机里，是从你 C 的三级测试纪律 + wg-core 思想共同得到的结论：**越多逻辑能在 PC 上测，开发效率和稳定性越高。**

---

## 5. 多外设 / 共享总线怎么扩展

「新增很多外设、分模块可复用」时，下面三件事是从 1 个外设到 N 个时一定会撞上的（详见 `blink` 的演进笔记）：

1. **外部器件驱动一律泛型于 `embedded-hal(-async)` trait**（如 `I2c`、`SpiDevice`），不要写死 `embassy_stm32` 具体类型——这就是把你 C 的 ops 表思想用 trait 重新表达，且免费获得跨芯片复用 + host 可测。
2. **共享总线**：多个 I2C/SPI 器件挂同一条总线时，用 `embassy-sync` 的 `Mutex` 包总线，再用 `embassy-embedded-hal` 的 `I2cDevice` / `SpiDevice` 给每个器件发「子句柄」（它又实现了 trait，驱动层无感）。**加第二个 I2C 器件当天就会遇到。**
3. **任务间解耦**：actuator/sensor 之间不要互相直接调用，统一走 `Channel`（事件流）或 `Signal`（最新状态）——这正是你 C 里 handler queue 解耦意图的 Rust 版。

---

## 6. 诚实提醒：参照工程目前是脚手架，别抄 bug

`qiyuan_lx` 这份 C **现在还是脚手架阶段**，迁移时借意图、别连 bug 一起搬（这些在 Rust 里要么编译期就被挡掉，要么自然消失）：

| C 现状 | Rust 里的结果 |
|---|---|
| `led_on/off` 是 `printf` 占位，**没真接 GPIO** | board.rs 注入真实 `Output`，自然解决 |
| `led_event_t.index` 声明成 `const` 又漏赋值，**恒为 0**（多 LED 调度没真生效） | 消息字段**必填**，编译器强制，不可能漏 |
| worker 用 `vTaskDelay(1000)` 轮询队列，**每条命令最多 1s 延迟** | `channel.receive().await` 零延迟 |
| 线程在队列创建之前 spawn，靠 `osDelay(1000)` 硬等，有竞态 | `Channel` 是 static，task 在资源就绪后 spawn，无此竞态 |
| blink 算法在 driver / handler **抄了两份** | 一个 `async fn blink` 复用 |
| `is_inited` + `INIT_PATTERN` 哨兵手工状态机 | `Option` / 所有权 / 类型系统 |

---

## 7. blink 的目标结构（把这套落进去）

```text
src/
  main.rs        装配：init board → 分发资源 → spawn tasks
  board.rs       唯一平台适配层（= system_adaption）：芯片+引脚+时钟 → 返回句柄
  drivers/       外部器件驱动，泛型于 trait，可复用 + host 可测（= driver 层）
    mod.rs
    led.rs
  app/           与硬件无关的状态机/策略，host 可测（按需，有真实状态机再建）
  tasks/         Embassy task：等命令 → 调 driver/app → 驱动硬件（= handler 的调度职责）
    mod.rs
  runtime.rs     任务间通信：channel / signal（按需）
```

层与 C 的对应：

```text
C: app ──cmd──> handler(注册表+worker+queue) ──> driver(ops 注入) ──> system_adaption(HAL)
Rust: app ─cmd→ Channel ─→ led_task(Embassy executor) ─→ Led<P>(泛型) ─→ board.rs(具体引脚)
                 └────────── handler 这一层大半被 Embassy 运行时吸收 ──────────┘
```

> **关于「config」**：不要建一个大 `config.rs` 把所有常量堆一起。常量就近放在用它的模块里（编译期 `const`，零成本）；只有跨模块/系统级的（固件版本、板型）才进一个很短的 `config.rs`；运行时可改、要存 Flash 的是 settings，归 `runtime.rs`，别和 `const` 混。

---

## 8. 判断迁移是否到位的标准（自检清单）

不要用「文件变多了」判断好坏，用这些问题：

1. 换 LED 引脚 / 极性时，是否**只改 `board.rs`**？
2. 写新器件驱动时，是否**只依赖 `embedded-hal` trait**、不出现 `embassy_stm32` 具体类型？
3. 该驱动能否**喂 mock 在 PC 上 `cargo test`**，不上板？
4. 加第二个同总线器件时，是否用 `I2cDevice`/`SpiDevice` 而**没去碰已有驱动**？
5. 任务之间是否**通过 channel/signal** 通信，而非互相直接调用？
6. 代码里是否**已经没有** `is_inited` 哨兵、手写注册表、`#ifdef` 分叉、轮询 worker 这些 C 残留？
7. 构造一个驱动是否是 `new() -> Self`/`Result`，**不存在半初始化**状态？

答案越多是「是」，说明你不是把 C 翻译成了 Rust，而是真正用 Rust 的方式表达了同一套架构思想。

---

## 9. 下一步

建议的落地顺序（小步、不牺牲简洁）：

```text
第一步：drivers/led.rs —— 泛型 Led<P>，board.rs 注入 PC13，跑通点灯（替换现有 tasks.rs 里的裸 toggle）
第二步：给 Led 写一个 host 单元测试（embedded-hal-mock），证明可脱硬件测
第三步：加 Channel + BlinkCmd，让「下命令」与「执行」解耦（handler 的 Rust 版）
第四步：以同样的 trait 模式接入第一个真实外设（按键 / I2C 传感器 / 串口）
第五步：器件变多后，引入共享总线（I2cDevice/SpiDevice）与事件解耦
```

> 你这套 C 的「魂」——平台解耦、面向接口、消息解耦、可测——**全部值得带走**，而且和 `blink` 既定的 board.rs / trait 驱动 / 可测 core / channel 解耦方向完全收敛。要带走的是思想；它的「肉」（vtable、注册表、`is_inited`、worker、`#ifdef`）在 Rust 里都有更安全的原生替代，留在 C 里就好。
