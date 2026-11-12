#![windows_subsystem = "windows"]

mod app;
mod ui;

use std::path::PathBuf;
use std::sync::mpsc;

use pdf_catcher::single_instance::{self, Role};

fn main() -> eframe::Result<()> {
    // File dari klik kanan "Buka dengan PDF Catcher" atau yang diseret ke ikon.
    let cwd = std::env::current_dir().unwrap_or_default();
    let args: Vec<PathBuf> = std::env::args_os()
        .skip(1)
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { cwd.join(p) })
        .collect();

    let role = single_instance::acquire(&args);
    if matches!(role, Role::Forwarded) {
        return Ok(());
    }

    let (in_tx, in_rx) = mpsc::channel::<Vec<PathBuf>>();

    let icon = image_icon();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("PDF Catcher")
        .with_inner_size([980.0, 720.0])
        .with_min_inner_size([760.0, 560.0])
        .with_drag_and_drop(true);
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        centered: true,
        renderer: eframe::Renderer::Wgpu,
        wgpu_options: wgpu_options(),
        ..Default::default()
    };

    eframe::run_native(
        "PDF Catcher",
        options,
        Box::new(move |cc| {
            if matches!(role, Role::Primary) {
                let ctx = cc.egui_ctx.clone();
                single_instance::serve(move |paths| {
                    let _ = in_tx.send(paths);
                    ctx.request_repaint();
                });
            }
            Ok(Box::new(app::App::new(cc, args, in_rx)))
        }),
    )
}

/// Menggambar lewat DirectX 12 (atau Vulkan), bukan OpenGL.
///
/// Di laptop dengan dua GPU (mis. AMD terintegrasi + NVIDIA) dan layar refresh tinggi,
/// penukaran buffer OpenGL bisa sesaat menampilkan buffer kosong sehingga seluruh jendela
/// berkedip setiap kali kursor bergerak. Kita juga memilih GPU terintegrasi (yang tersambung
/// ke layar laptop) agar frame tidak perlu disalin antar-GPU. OpenGL hanya cadangan terakhir.
fn wgpu_options() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup};
    use eframe::wgpu;
    use std::sync::Arc;

    let mut config = WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(setup) = &mut config.wgpu_setup {
        setup.instance_descriptor.backends = wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL;
        setup.power_preference = wgpu::PowerPreference::LowPower;
        setup.native_adapter_selector = Some(Arc::new(|adapters, surface| {
            let rank = |a: &wgpu::Adapter| {
                let info = a.get_info();
                let backend = match info.backend {
                    wgpu::Backend::Dx12 => 0,
                    wgpu::Backend::Vulkan => 1,
                    wgpu::Backend::Gl => 2,
                    _ => 3,
                };
                let device = match info.device_type {
                    wgpu::DeviceType::IntegratedGpu => 0,
                    wgpu::DeviceType::DiscreteGpu => 1,
                    wgpu::DeviceType::VirtualGpu | wgpu::DeviceType::Other => 2,
                    wgpu::DeviceType::Cpu => 3,
                };
                (backend, device)
            };
            adapters
                .iter()
                .filter(|a| surface.map_or(true, |s| a.is_surface_supported(s)))
                .min_by_key(|a| rank(a))
                .cloned()
                .ok_or_else(|| "Tidak ada kartu grafis yang bisa menampilkan jendela ini.".to_string())
        }));
    }
    config
}

fn image_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/icon-256.png"));
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}
