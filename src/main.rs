#![windows_subsystem = "windows"]

mod app;
mod error;
mod file;
mod graph;

use std::sync::Arc;

use app::GraphApp;
use eframe::{
    NativeOptions,
    egui::{self, FontData, IconData, ViewportBuilder},
};

fn main() {
    let app = GraphApp::default();

    let native_options = NativeOptions {
        centered: true,
        viewport: ViewportBuilder::default()
            .with_inner_size((800.0, 600.0))
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Better KT-SQEP",
        native_options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "NotoSansSC-Regular".to_string(),
                Arc::new(FontData::from_static(include_bytes!(
                    "../assets/NotoSansSC-Regular.ttf"
                ))),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "NotoSansSC-Regular".to_string());
            cc.egui_ctx.set_fonts(fonts);
            cc.egui_ctx.set_visuals(egui::Visuals::light());

            Ok(Box::new(app))
        }),
    )
    .unwrap();
}

fn load_icon() -> IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let icon = include_bytes!("../assets/zdc.png");
        let image = image::load_from_memory(icon)
            .expect("Failed to open icon path")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
