//! English / Vietnamese UI strings. `{}` placeholders are filled with [`fill`].

use crate::config::Language;

macro_rules! strings {
    ($($key:ident: $en:expr, $vi:expr;)*) => {
        pub struct Strings { $(pub $key: &'static str,)* }
        pub static EN: Strings = Strings { $($key: $en,)* };
        pub static VI: Strings = Strings { $($key: $vi,)* };
    };
}

strings! {
    // Navigation
    nav_dashboard: "Dashboard", "Tổng quan";
    nav_curve: "Fan curve", "Đường cong";
    nav_drivers: "Drivers & tips", "Driver & mẹo";
    nav_settings: "Settings", "Cài đặt";
    nav_about: "About", "Giới thiệu";

    // Dashboard
    cpu: "CPU", "CPU";
    gpu: "GPU", "GPU";
    cpu_fan: "CPU fan", "Quạt CPU";
    gpu_fan: "GPU fan", "Quạt GPU";
    gpu_off: "off", "tắt";
    history: "Last 5 minutes", "5 phút gần nhất";
    legend_cpu: "CPU °C", "CPU °C";
    legend_gpu: "GPU °C", "GPU °C";
    legend_fan: "Fan %", "Quạt %";
    control_mode: "Control mode", "Chế độ điều khiển";
    mode_auto: "Auto", "Tự động";
    mode_fixed: "Fixed", "Cố định";
    mode_curve: "Curve", "Đường cong";
    mode_auto_desc: "The EC firmware controls the fans with its built-in curve (60 °C = 40 % … 80 °C = 100 %).",
        "Firmware EC tự điều khiển quạt theo đường cong gốc (60 °C = 40 % … 80 °C = 100 %).";
    mode_fixed_desc: "Both fans run at one constant speed.", "Cả hai quạt chạy ở một tốc độ cố định.";
    mode_curve_desc: "Speed follows your curve, based on the hotter of CPU and GPU.",
        "Tốc độ theo đường cong của bạn, dựa trên nhiệt độ cao nhất giữa CPU và GPU.";
    fixed_low_warning: "Low speeds can overheat the CPU under load. The safety limit still applies.",
        "Tốc độ thấp có thể làm CPU quá nhiệt khi tải nặng. Giới hạn an toàn vẫn được áp dụng.";
    edit_curve: "Edit curve", "Chỉnh đường cong";
    preset: "Preset", "Mẫu có sẵn";
    preset_silent: "Silent", "Êm";
    preset_balanced: "Balanced", "Cân bằng";
    preset_performance: "Performance", "Hiệu năng";
    preset_firmware: "Firmware", "Firmware";

    // Status
    st_connecting: "Connecting to the EC…", "Đang kết nối EC…";
    st_auto: "Firmware in control", "Firmware đang điều khiển";
    st_manual: "Manual · {} %", "Thủ công · {} %";
    st_overtemp: "Safety: {} °C — handed back to firmware", "An toàn: {} °C — đã trả quyền cho firmware";
    st_notemp: "Safety: temperature unavailable — handed back to firmware",
        "An toàn: không đọc được nhiệt độ — đã trả quyền cho firmware";
    st_error: "EC error: {}", "Lỗi EC: {}";
    legacy_note: "Older EC firmware ({}): no fan-mode command, so the PWM is rewritten every 40 ms. Updating the System76 firmware gives smoother control.",
        "Firmware EC cũ ({}): không có lệnh fan mode nên PWM được ghi lại mỗi 40 ms. Cập nhật firmware System76 sẽ điều khiển mượt hơn.";

    // Connection failure
    fail_title: "Cannot access the embedded controller", "Không truy cập được bộ điều khiển nhúng (EC)";
    fail_hint_dll: "Put inpoutx64.dll next to the executable (it is included in the release zip).",
        "Đặt inpoutx64.dll cạnh file exe (có sẵn trong file zip release).";
    fail_hint_admin: "The program must run as administrator.", "Chương trình phải chạy với quyền Administrator.";
    fail_hint_hvci: "If Memory Integrity (Core isolation) is on, Windows may block the InpOut driver.",
        "Nếu bật Memory Integrity (Core isolation), Windows có thể chặn driver InpOut.";
    fail_hint_model: "Only System76 laptops with System76 Open Firmware (coreboot + System76 EC) are supported.",
        "Chỉ hỗ trợ laptop System76 dùng System76 Open Firmware (coreboot + System76 EC).";
    retry: "Retry", "Thử lại";
    download_inpout: "Download InpOut", "Tải InpOut";

    // Curve page
    curve_title: "Fan curve", "Đường cong quạt";
    curve_help: "Drag points · double-click to add · right-click to remove",
        "Kéo điểm · nhấp đúp để thêm · chuột phải để xoá";
    curve_now: "now", "hiện tại";
    safety: "Safety", "An toàn";
    safety_temp: "Hand back to firmware above", "Trả quyền cho firmware khi vượt";
    min_speed: "Minimum speed", "Tốc độ tối thiểu";
    ramp_down: "Slow down by at most", "Giảm tối đa";
    per_sec: "%/s", "%/giây";
    reset: "Reset", "Đặt lại";

    // Drivers page
    drv_required: "Required by this tool", "Cần cho tool này";
    drv_inpout: "InpOut port I/O driver", "Driver cổng I/O InpOut";
    drv_inpout_desc: "Lets the app talk to the EC. Bundled as inpoutx64.dll; the DLL installs its signed kernel driver on first run.",
        "Cho phép app giao tiếp với EC. Đi kèm dưới dạng inpoutx64.dll; DLL tự cài driver kernel (đã ký) ở lần chạy đầu.";
    drv_ok: "Working", "Hoạt động";
    drv_missing: "Not working", "Không hoạt động";
    drv_official: "Official System76 drivers", "Driver chính thức của System76";
    drv_official_desc: "System76 publishes Windows drivers for Open Firmware machines. Install these for model {}:",
        "System76 cung cấp driver Windows cho các máy Open Firmware. Nên cài cho model {}:";
    drv_official_unknown: "Find your model in the System76 Windows drivers repository.",
        "Tìm model của bạn trong repo driver Windows của System76.";
    drv_open_page: "Open driver page", "Mở trang driver";
    drv_others: "Everything else (audio, card reader, …) comes from Windows Update.",
        "Các driver còn lại (âm thanh, đầu đọc thẻ, …) Windows Update sẽ tự cài.";
    drv_nvidia: "NVIDIA GPU: install the latest driver from nvidia.com.", "GPU NVIDIA: cài driver mới nhất từ nvidia.com.";
    drv_nvidia_btn: "NVIDIA drivers", "Driver NVIDIA";
    drv_devices: "Devices without a driver", "Thiết bị chưa có driver";
    drv_scan: "Scan", "Quét";
    drv_scanning: "Scanning…", "Đang quét…";
    drv_none: "All devices have a driver.", "Mọi thiết bị đều đã có driver.";
    drv_dev_s76: "System76 ACPI device. It only has a Linux driver (keyboard backlight, hotkeys); safe to ignore, the EC handles hotkeys itself.",
        "Thiết bị ACPI của System76. Chỉ có driver cho Linux (đèn bàn phím, phím nóng); có thể bỏ qua, EC tự xử lý phím nóng.";
    drv_dev_coreboot: "coreboot table device, used only by Linux. Safe to ignore.",
        "Thiết bị bảng coreboot, chỉ Linux dùng. Có thể bỏ qua.";
    drv_dev_unknown: "Check Windows Update or the System76 driver page above.",
        "Kiểm tra Windows Update hoặc trang driver System76 ở trên.";
    drv_code: "code {}", "mã {}";
    tips: "Tips for cooler and quieter fans", "Mẹo để máy mát và quạt êm hơn";
    tip_power: "Use the Balanced power plan and set Processor power management → Maximum processor state to 99 %. This turns off Turbo Boost and lowers temperatures a lot.",
        "Dùng power plan Balanced và đặt Processor power management → Maximum processor state = 99 %. Cách này tắt Turbo Boost và giảm nhiệt rất nhiều.";
    tip_power_btn: "Open Power Options", "Mở Power Options";
    tip_fn1: "Fn + 1 toggles the firmware's \"max fan\" mode. If the fans are always at 100 %, press it once.",
        "Fn + 1 bật/tắt chế độ \"quạt tối đa\" của firmware. Nếu quạt luôn 100 %, hãy bấm thử một lần.";
    tip_background: "Check Task Manager for background apps that keep a core busy; a single busy core keeps the CPU boosting and hot.",
        "Kiểm tra Task Manager xem có app chạy nền chiếm một nhân CPU liên tục không; chỉ một nhân bận cũng khiến CPU boost và nóng.";

    // Settings
    set_general: "General", "Chung";
    set_language: "Language", "Ngôn ngữ";
    set_lang_system: "System", "Theo hệ thống";
    set_theme: "Theme", "Giao diện";
    set_dark: "Dark", "Tối";
    set_light: "Light", "Sáng";
    set_autostart: "Start with Windows (minimized to tray)", "Chạy cùng Windows (thu nhỏ xuống khay)";
    set_close_to_tray: "Closing the window keeps the app running in the tray", "Đóng cửa sổ vẫn chạy dưới khay hệ thống";
    set_config_file: "Configuration file", "File cấu hình";
    set_open_folder: "Open folder", "Mở thư mục";
    set_autostart_error: "Could not change autostart: {}", "Không đổi được chế độ tự khởi động: {}";

    // About
    about_desc: "Fan control for System76 laptops with System76 Open Firmware, running Windows.",
        "Điều khiển quạt cho laptop System76 dùng System76 Open Firmware khi chạy Windows.";
    about_disclaimer: "Community project, not affiliated with or endorsed by System76. Use at your own risk.",
        "Dự án cộng đồng, không liên kết hay được System76 bảo trợ. Tự chịu rủi ro khi sử dụng.";
    about_how: "Talks to the open-source System76 EC firmware through its command interface (I/O ports 0xE00) and reads temperatures from the EC's ACPI registers.",
        "Giao tiếp với firmware EC mã nguồn mở của System76 qua giao diện lệnh (cổng I/O 0xE00) và đọc nhiệt độ từ thanh ghi ACPI của EC.";
    about_ec: "EC", "EC";
    about_source: "Source code", "Mã nguồn";
    about_licenses: "Licenses: this app MIT · InpOut MIT (Phil Gibbons, highrez.co.uk)",
        "Giấy phép: app này MIT · InpOut MIT (Phil Gibbons, highrez.co.uk)";

    // Tray
    tray_show: "Show window", "Mở cửa sổ";
    tray_quit: "Quit (fans back to auto)", "Thoát (trả quạt về tự động)";
}

pub fn strings(lang: Language) -> &'static Strings {
    match lang {
        Language::En => &EN,
        Language::Vi => &VI,
        Language::System => {
            if system_is_vietnamese() {
                &VI
            } else {
                &EN
            }
        }
    }
}

/// Replaces `{}` placeholders in order.
pub fn fill(template: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut parts = template.split("{}");
    if let Some(first) = parts.next() {
        out.push_str(first);
    }
    for (i, part) in parts.enumerate() {
        if let Some(a) = args.get(i) {
            out.push_str(&a.to_string());
        }
        out.push_str(part);
    }
    out
}

fn system_is_vietnamese() -> bool {
    // SAFETY: trivially safe.
    let lang = unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() };
    const LANG_VIETNAMESE: u16 = 0x2A;
    lang & 0x3FF == LANG_VIETNAMESE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_placeholders() {
        assert_eq!(fill("a {} b {}", &[&1, &"x"]), "a 1 b x");
        assert_eq!(fill("none", &[]), "none");
        assert_eq!(fill("{} only", &[]), " only");
    }
}
