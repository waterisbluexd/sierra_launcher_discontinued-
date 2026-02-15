use iced::widget::{container, text, column, row, stack, Space};
use iced::{Element, Border, Color, Length, Alignment, Font};
use crate::utils::theme::Theme;
use crate::Message;
use sysinfo::{System, Disks, Networks};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(serde::Deserialize)]
struct RocmSmiCard {
    #[serde(rename = "GPU use (%)")]
    gpu_use: Option<String>,
}

enum GpuType {
    Nvidia,
    Amd,
    None,
}

struct GpuManager {
    gpu_type: GpuType,
    cache: Mutex<(Vec<f32>, Instant)>,
}

impl GpuManager {
    fn new() -> Self {
        let gpu_type = if Self::command_exists("nvidia-smi") {
            GpuType::Nvidia
        } else if Self::command_exists("rocm-smi") {
            GpuType::Amd
        } else {
            GpuType::None
        };

        Self {
            gpu_type,
            cache: Mutex::new((vec![0.0, 0.0], Instant::now() - Duration::from_secs(10))),
        }
    }

    fn command_exists(cmd: &str) -> bool {
        let output = std::process::Command::new("which")
            .arg(cmd)
            .output();

        if let Ok(output) = output {
            output.status.success()
        } else {
            false
        }
    }

    fn get_gpu_usage_cached(&self) -> Vec<f32> {
        let mut guard = self.cache.lock().unwrap();
        if guard.1.elapsed() > Duration::from_secs(4) {
            guard.0 = self.get_gpu_usage();
            guard.1 = Instant::now();
        }
        guard.0.clone()
    }

    fn get_gpu_usage(&self) -> Vec<f32> {
        match self.gpu_type {
            GpuType::Nvidia => self.get_nvidia_usage(),
            GpuType::Amd => self.get_amd_usage(),
            GpuType::None => vec![0.0, 0.0],
        }
    }

    fn get_nvidia_usage(&self) -> Vec<f32> {
        let mut gpu_usages = vec![0.0, 0.0];
        let output = std::process::Command::new("timeout")
            .args([
                "1",
                "nvidia-smi",
                "--query-gpu=utilization.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output();

        if let Ok(o) = output {
            if let Ok(text) = String::from_utf8(o.stdout) {
                for (i, line) in text.lines().take(2).enumerate() {
                    if let Ok(v) = line.trim().parse::<f32>() {
                        gpu_usages[i] = v;
                    }
                }
            }
        }
        gpu_usages
    }

    fn get_amd_usage(&self) -> Vec<f32> {
        let mut gpu_usages = vec![0.0, 0.0];
        let output = std::process::Command::new("timeout")
            .args(["1", "rocm-smi", "--show-usage", "--json"])
            .output();

        if let Ok(o) = output {
            if let Ok(text) = String::from_utf8(o.stdout) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(cards_map) = json.as_object() {
                        let cards = cards_map.into_iter().filter(|(k, _)| k.starts_with("card"));
                        for (i, (_, card_val)) in cards.take(2).enumerate() {
                            if let Ok(card_info) = serde_json::from_value::<RocmSmiCard>(card_val.clone()) {
                                if let Some(use_percent) = card_info.gpu_use {
                                    if let Ok(v) = use_percent.trim().parse::<f32>() {
                                        gpu_usages[i] = v;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        gpu_usages
    }
}


pub struct SystemPanel {
    metrics: Arc<Mutex<Option<SystemMetrics>>>,
    started: bool,
}

#[derive(Clone)]
pub struct SystemMetrics {
    pub cpu_usage: f32,
    pub mem_usage: f32,
    pub net_usage: f32,
    pub disk_usage: f32,
    pub gpu_usage: f32,
    pub gpu1_usage: f32,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            mem_usage: 0.0,
            net_usage: 0.0,
            disk_usage: 0.0,
            gpu_usage: 0.0,
            gpu1_usage: 0.0,
        }
    }
}

impl SystemPanel {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(None)),
            started: false,
        }
    }

    pub fn start(&mut self) {
        if self.started {
            return;
        }
        self.started = true;

        let metrics = Arc::clone(&self.metrics);

        thread::spawn(move || {
            let mut sys = System::new_all();
            let mut networks = Networks::new_with_refreshed_list();
            let mut disks = Disks::new_with_refreshed_list();
            let gpu_manager = Arc::new(GpuManager::new());

            *metrics.lock().unwrap() = Some(SystemMetrics::default());

            loop {
                sys.refresh_cpu_usage();
                sys.refresh_memory();
                networks.refresh();
                disks.refresh();

                let mut guard = metrics.lock().unwrap();
                if let Some(ref mut m) = *guard {
                    m.cpu_usage = sys.global_cpu_usage();

                    let total = sys.total_memory();
                    m.mem_usage = if total > 0 {
                        (sys.used_memory() as f64 / total as f64 * 100.0) as f32
                    } else {
                        0.0
                    };

                    let (used, total) = disks.list().iter().fold(
                        (0u64, 0u64),
                        |(u, t), d| {
                            let total = d.total_space();
                            let used = total - d.available_space();
                            (u + used, t + total)
                        },
                    );

                    m.disk_usage = if total > 0 {
                        (used as f64 / total as f64 * 100.0) as f32
                    } else {
                        0.0
                    };

                    let net: u64 = networks
                        .iter()
                        .map(|(_, n)| n.received() + n.transmitted())
                        .sum();

                    m.net_usage = (net as f64 / 10_000_000.0 * 100.0)
                        .min(100.0) as f32;

                    let gpus = gpu_manager.get_gpu_usage_cached();
                    m.gpu_usage = gpus[0];
                    m.gpu1_usage = gpus[1];
                }

                drop(guard);
                thread::sleep(Duration::from_secs(2));
            }
        });
    }
}

#[inline]
fn vertical_bar<'a>(
    label: &'a str,
    value: f32,
    theme: &'a Theme,
    font: Font,
) -> Element<'a, Message> {
    const BAR_WIDTH: f32 = 20.0;

    let percentage_text = text(format!("{:.0}%", value))
        .size(12)
        .font(font)
        .color(Color::WHITE)
        .width(Length::Fill)
        .center();

    let bar_height_ratio = (value / 100.0).clamp(0.0, 1.0);

    let filled_portion = (bar_height_ratio * 1000.0).round() as u16;
    let empty_portion = 1000u16.saturating_sub(filled_portion);

    let bar_visual = container(
        column![
            container(Space::new())
                .width(Length::Fixed(BAR_WIDTH))
                .height(if empty_portion > 0 {
                    Length::FillPortion(empty_portion)
                } else {
                    Length::Fixed(0.0)
                })
                .style(move |_| container::Style {
                    background: Some(theme.color11.into()),
                    ..Default::default()
                }),
            container(Space::new())
                .width(Length::Fixed(BAR_WIDTH))
                .height(if filled_portion > 0 {
                    Length::FillPortion(filled_portion)
                } else {
                    Length::Fixed(0.0)
                })
                .style(move |_| container::Style {
                    background: Some(theme.color6.into()),
                    ..Default::default()
                }),
        ]
        .spacing(0)
        .width(Length::Fixed(BAR_WIDTH))
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill);

    column![
        percentage_text,
        bar_visual,
        text(label)
            .size(12)
            .font(font)
            .color(theme.color3)
            .width(Length::Fill)
            .center()
    ]
    .spacing(4)
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Center)
    .into()
}

pub fn system_panel_view<'a>(
    system_panel: &'a SystemPanel,
    theme: &'a Theme,
    bg_with_alpha: Color,
    font: iced::Font,
    font_size: f32,
) -> Element<'a, Message> {
    let metrics_guard = system_panel.metrics.lock().unwrap();

    if metrics_guard.is_none() {
        drop(metrics_guard);

        return container(
            container(
                stack![
                    container(
                        container(
                            text("Loading system info...")
                                .font(font)
                                .size(font_size)
                                .color(theme.color6)
                                .center()
                        )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .padding(iced::padding::top(25))
                        .style(move |_| container::Style {
                            background: None,
                            border: Border {
                                color: theme.color3,
                                width: 2.0,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        })
                    )
                    .padding(iced::padding::top(15))
                    .width(Length::Fill)
                    .height(Length::Fill),

                    container(
                        container(
                            text(" System ")
                                .color(theme.color6)
                                .font(font)
                                .size(font_size)
                        )
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .style(move |_| container::Style {
                            background: Some(bg_with_alpha.into()),
                            ..Default::default()
                        })
                    )
                    .padding(iced::padding::left(8).top(5))
                    .width(Length::Shrink)
                    .height(Length::Shrink),
                ]
            )
            .width(Length::Fill)
            .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::FillPortion(1))
        .into();
    }

    let metrics = metrics_guard.as_ref().unwrap().clone();
    drop(metrics_guard);

    let metrics_data = [
        ("CPU", metrics.cpu_usage),
        ("MEM", metrics.mem_usage),
        ("NET", metrics.net_usage),
        ("DISK", metrics.disk_usage),
        ("GPU0", metrics.gpu_usage),
        ("GPU1", metrics.gpu1_usage),
    ];

    let bars_row = row(
        metrics_data
            .iter()
            .map(|&(label, value)| vertical_bar(label, value, theme, font))
            .collect::<Vec<_>>()
    )
    .spacing(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(8);

    container(
        container(
            stack![
                container(
                    container(bars_row)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(iced::padding::top(25))
                        .center_x(Length::Fill)
                        .center_y(Length::Fill)
                        .style(move |_| container::Style {
                            background: None,
                            border: Border {
                                color: theme.color3,
                                width: 2.0,
                                radius: 0.0.into(),
                            },
                            ..Default::default()
                        })
                )
                .padding(iced::padding::top(15))
                .width(Length::Fill)
                .height(Length::Fill),

                container(
                    container(
                        text(" System ")
                            .color(theme.color6)
                            .font(font)
                            .size(font_size)
                    )
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .style(move |_| container::Style {
                        background: Some(bg_with_alpha.into()),
                        ..Default::default()
                    })
                )
                .padding(iced::padding::left(8).top(5))
                .width(Length::Shrink)
                .height(Length::Shrink),
            ]
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: None,
            ..Default::default()
        }),
    )
    .width(Length::Fill)
    .height(Length::FillPortion(1))
    .style(move |_| container::Style {
        background: None,
        ..Default::default()
    })
    .into()
}
