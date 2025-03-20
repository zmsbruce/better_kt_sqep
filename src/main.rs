mod app;
mod error;
mod graph;

use std::sync::Arc;

use app::GraphApp;
use eframe::{
    NativeOptions,
    egui::{self, FontData},
};
use graph::{AddonEntityType, DistinctEntityType, KnowledgeGraph, Relation};

fn main() {
    let knowledge_graph = create_knowledge_graph().unwrap();
    let mut app = GraphApp::default();
    app.graph = knowledge_graph;

    eframe::run_native(
        "test",
        NativeOptions::default(),
        Box::new(|cc| {
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

fn create_knowledge_graph() -> Result<KnowledgeGraph, Box<dyn std::error::Error>> {
    let mut knowledge_graph = KnowledgeGraph::new(0);

    let id_1 = knowledge_graph.add_entity(
        "什么是计算思维".to_string(),
        DistinctEntityType::KnowledgeArena,
        &[AddonEntityType::Thinking],
        (100.0, 300.0),
    );
    let id_2 = knowledge_graph.add_entity(
        "典型的计算思维".to_string(),
        DistinctEntityType::KnowledgePoint,
        &[
            AddonEntityType::Thinking,
            AddonEntityType::Example,
            AddonEntityType::Question,
        ],
        (100.0, 100.0),
    );
    let id_3 = knowledge_graph.add_entity(
        "小白鼠检验毒水瓶问题,怎样求解？".to_string(),
        DistinctEntityType::KnowledgeDetail,
        &[
            AddonEntityType::Practice,
            AddonEntityType::Example,
            AddonEntityType::Question,
        ],
        (100.0, 500.0),
    );
    let id_4 = knowledge_graph.add_entity(
        "水瓶编号：由十进制编号到二进制编号".to_string(),
        DistinctEntityType::KnowledgeDetail,
        &[
            AddonEntityType::Practice,
            AddonEntityType::Example,
            AddonEntityType::Thinking,
        ],
        (350.0, 500.0),
    );
    knowledge_graph.add_edge(id_1, id_2, Relation::Contain)?;
    knowledge_graph.add_edge(id_1, id_3, Relation::Contain)?;
    knowledge_graph.add_edge(id_3, id_4, Relation::Order)?;

    Ok(knowledge_graph)
}
