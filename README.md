# S76 Fan Control

Fan control for **System76 laptops running Windows** with **System76 Open Firmware**
(coreboot + the open-source [System76 EC](https://github.com/system76/ec)).

On Linux, System76 machines are handled by `system76-power` and the `system76-acpi`
driver. Windows has no equivalent, so the fans run only on the EC's built-in curve. That
curve reaches 100 % at 80 °C, which a modern Intel HX CPU passes even at idle under
Windows. This tool lets you choose how the fans behave.

![Dashboard](docs/screenshots/dashboard.png)

| Fan curve editor | Drivers & tips |
|---|---|
| ![Curve](docs/screenshots/curve.png) | ![Drivers](docs/screenshots/drivers.png) |

> **Disclaimer.** This is a community project. It is not affiliated with or endorsed by
> System76. It writes to your embedded controller, so you use it at your own risk.

## Features

- **Three control modes**
  - **Auto**: the EC firmware controls the fans (the default behaviour).
  - **Fixed**: one constant speed for both fans.
  - **Curve**: speed follows a temperature curve that you edit. The curve uses the hotter
    of CPU and GPU. Speed rises immediately and falls gradually.
- **Interactive curve editor**: drag points, double-click to add a point, right-click to
  remove one. Presets: Silent, Balanced, Performance, Firmware.
- **Live dashboard**: CPU/GPU temperature, RPM and PWM of both fans, and a 5-minute history.
- **Safety first**
  - Control goes back to the EC above a configurable temperature (default 92 °C). It is
    taken again only after a 5 °C drop.
  - Control goes back to the EC if the CPU temperature cannot be read 5 times in a row.
  - Control goes back to the EC when the app exits, and when the control thread panics.
  - A minimum fan speed is enforced in manual modes.
- **Drivers & tips page**
  - Links to the official [System76 Windows drivers](https://github.com/system76/windows-drivers)
    for your model.
  - Lists devices that have no driver, and explains the known harmless ones.
  - Tips for a cooler and quieter machine.
- **System tray**: switch modes, see temperatures in the tooltip, and close the window to
  the tray.
- **Start with Windows**: uses a Task Scheduler logon task, so there is no UAC prompt at boot.
- **English and Tiếng Việt**, dark and light themes.
- Native, small and fast: a single ~6.5 MB Rust executable built with
  [egui](https://github.com/emilk/egui), with no runtime to install.

## Supported hardware

The app works on System76 laptops that run **System76 Open Firmware**. When it starts, it
probes the EC and refuses to run on anything else.

It was developed and tested on an **Adder WS (addw3)**, EC firmware `2023-09-19_16ef69c`.
Other Open Firmware models use the same EC interface and should work. Reports are welcome.

| EC firmware | Behaviour |
|---|---|
| 2025 and newer (has `FAN_SET_MODE`) | The EC is switched to manual PWM mode. Control is smooth and stable. |
| Older | The EC rewrites the PWM on every fan tick (250 ms–1 s), so the app rewrites it every 40 ms. This works well: on addw3 the EC "wins" about 17 % of ticks, with no audible effect. |

Machines with proprietary (Insyde/AMI) firmware are **not** supported.

## Installation

1. Download `S76FanControl-<version>-windows-x64.zip` from
   [Releases](../../releases) and extract it to a permanent folder, for example
   `C:\Program Files\S76FanControl`.
2. Keep `inpoutx64.dll` next to `S76FanControl.exe`.
3. Run `S76FanControl.exe` and accept the UAC prompt. Administrator rights are required
   for port I/O.
4. Optional: in **Settings**, enable **Start with Windows**.

### Why InpOut?

Windows does not allow normal programs to access I/O ports. The app uses
[InpOut32/x64](https://www.highrez.co.uk/downloads/inpout32/) (MIT, by Phil Gibbons), a
tiny DLL with an embedded, signed kernel driver that it installs on first use.

If **Memory Integrity** (*Windows Security → Device security → Core isolation*) is
enabled, Windows may refuse to load the driver. The dashboard then shows a "Cannot access
the embedded controller" card.

## Recommended drivers for Windows

From [system76/windows-drivers](https://github.com/system76/windows-drivers). The app
shows the right page for your model.

| Model | Package |
|---|---|
| Adder WS addw3/addw4, Bonobo WS bonw15, Oryx Pro oryp12, Serval WS serw13 | [Intel RPL-HX](https://github.com/system76/windows-drivers/tree/master/drivers/intel/rpl-hx): Chipset, Serial IO, Thunderbolt, Wi-Fi |
| addw5, bonw16, serw14 | [Intel ARL-HX](https://github.com/system76/windows-drivers/tree/master/drivers/intel/arl-hx) |
| Other models | see the [repository README](https://github.com/system76/windows-drivers) |

Also install the latest NVIDIA driver from nvidia.com. Windows Update provides the rest.

Device Manager may show devices with "no driver" that you can safely ignore:

- `ACPI\17761776` is the System76 ACPI device. Its driver exists only for Linux (keyboard
  backlight and hotkeys). The EC handles the hotkeys itself.
- `ACPI\BOOT0000` is the coreboot table device, which only Linux uses.

## Tips for a quieter machine

- Use the **Balanced** power plan. Set *Processor power management → Maximum processor
  state* to **99 %**. This disables Turbo Boost and lowers temperatures dramatically.
- **Fn + 1** toggles the firmware's "max fan" mode. If the fans are stuck at 100 %, press
  it once.
- Look in Task Manager for background processes that keep one core busy. One busy core is
  enough to keep the CPU boosting and hot.

## Diagnostics

```
S76FanControl.exe --diag          # read-only report → %APPDATA%\S76FanControl\diag.txt
S76FanControl.exe --diag --test   # also drives the fans to 30 % then 100 %, then back to auto
```

Settings are stored in `%APPDATA%\S76FanControl\config.toml`. If a `config.toml` sits next
to the executable, the app uses that file instead (portable mode).

## How it works

| What | How |
|---|---|
| EC commands | SMFI command interface at I/O ports `0xE00–0xEFF`: `PROBE`, `BOARD`, `VERSION`, `FAN_GET_PWM` (7), `FAN_SET_PWM` (8), `FAN_GET_RPM` (22), `FAN_GET_MODE` (23), `FAN_SET_MODE` (24) — see [`smfi.c`](https://github.com/system76/ec/blob/master/src/app/main/smfi.c) |
| Temperatures | EC ACPI registers through ports `0x62/0x66`: `0x07` CPU (PECI), `0xCD` dGPU |
| RPM on old firmware | Raw tachometer from ACPI `0xD0..0xD3`, converted like the firmware's `TACH_TO_RPM` |

Source layout:

```
src/
  hw/            InpOut loader, SMFI protocol, ACPI EC reads, FanHardware trait
  controller.rs  control thread + Governor (pure decision logic, unit-tested)
  curve.rs       curve evaluation, presets, sanitising
  config.rs      TOML settings
  drivers.rs     model → System76 driver page mapping
  platform.rs    single instance, autostart, device scan, Win32 helpers
  tray.rs        tray icon & menu
  ui/            egui app: theme, widgets, charts, pages
```

## Building from source

Requirements: the Rust stable MSVC toolchain and the Windows SDK (for `rc.exe`).

```
cargo build --release
cargo test
```

The build embeds the manifest (requires administrator, DPI aware) and the icon, which is
drawn in code (`src/icon_art.rs`). It also copies `vendor/inpout/inpoutx64.dll` next to the
executable.

To regenerate the screenshots, run `S76FanControl.exe --screenshot <dir>`. The app captures
each page as a BMP and exits.

## License

MIT, see [LICENSE](LICENSE). Third-party components:

- InpOut32/x64 © Phil Gibbons, MIT ([vendor/inpout/LICENSE.txt](vendor/inpout/LICENSE.txt)).
- The EC protocol is documented from [system76/ec](https://github.com/system76/ec)
  (GPL-3.0). No firmware code is included.

---

## Tiếng Việt

**S76 Fan Control** là tool điều khiển quạt cho laptop System76 chạy **Windows** với
**System76 Open Firmware**.

- Có 3 chế độ: **Tự động** (firmware điều khiển), **Cố định**, và **Đường cong** (kéo thả
  điểm, có sẵn các mẫu).
- Tự trả quyền cho firmware khi thoát app, khi vượt nhiệt độ an toàn (mặc định 92 °C),
  hoặc khi không đọc được nhiệt độ.
- Trang **Driver & mẹo** gợi ý driver chính thức của System76 cho đúng model, quét các
  thiết bị chưa có driver, và hướng dẫn giảm nhiệt.
- Cài đặt: giải nén file zip trong mục Releases, giữ `inpoutx64.dll` cạnh file exe, chạy
  với quyền Administrator. Có thể bật **Chạy cùng Windows** trong Cài đặt.
- Mẹo giảm nhiệt:
  - Dùng power plan **Balanced** và đặt *Maximum processor state* = **99 %** để tắt Turbo
    Boost.
  - **Fn + 1** bật/tắt chế độ quạt tối đa của firmware.
