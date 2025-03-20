use std::{collections::HashMap, time};

use eframe::{
    App,
    egui::{self, Color32, Context, FontFamily, FontId, Painter, Pos2, Rect, Stroke, Vec2},
    emath::Rot2,
};

use crate::{
    error::GraphError,
    graph::{AddonEntityType, DistinctEntityType, EntityNode, KnowledgeGraph, Relation},
};

const NODE_SIZE: Vec2 = Vec2::new(120.0, 90.0);

pub struct GraphApp {
    pub graph: KnowledgeGraph,

    // 上一次单机左键的信息
    last_click_time: time::Instant,
    last_click_pos: Pos2,

    // 将编辑数据持久化，避免每一帧重置
    editing_node: Option<u64>,
    editing_content: String,
    editing_distinct_type: DistinctEntityType,
    editing_addon_types: HashMap<AddonEntityType, bool>,

    // 选中的节点或边
    selected_node: Option<u64>,
    selected_edge: Option<(u64, u64)>,

    // 拖拽的节点
    dragging_node: Option<u64>,
    dragging_offset: Vec2,

    // 鼠标所在的节点或边
    hovered_node: Option<(u64, bool)>,
    hovered_edge: Option<(u64, u64)>,

    // 绘制边的起点
    edge_start_node: Option<u64>,
    edge_end_node: Option<u64>,
    current_relation: Relation,
}

impl Default for GraphApp {
    fn default() -> Self {
        Self {
            graph: KnowledgeGraph::new(100),
            last_click_pos: Pos2::new(-100.0, -100.0), // 初始化为一个不可能的位置
            last_click_time: time::Instant::now() - time::Duration::from_secs(1), // 初始化为一个不可能的时间
            editing_node: None,
            editing_content: String::new(),
            editing_distinct_type: DistinctEntityType::KnowledgeArena,
            editing_addon_types: HashMap::with_capacity(6),
            selected_node: None,
            selected_edge: None,
            dragging_node: None,
            dragging_offset: Vec2::ZERO,
            hovered_node: None,
            hovered_edge: None,
            edge_start_node: None,
            edge_end_node: None,
            current_relation: Relation::Contain,
        }
    }
}

impl App for GraphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // 获取 Painter 开始绘制
            let painter = ui.painter();

            {
                // 从图谱中获取当前快照
                let snapshot = self.graph.current_snapshot();

                // 先绘制边
                for ((from, to), relation) in snapshot.edges.iter() {
                    if let (Some(from_node), Some(to_node)) =
                        (snapshot.nodes.get(from), snapshot.nodes.get(to))
                    {
                        self.draw_edge(painter, from_node, to_node, *relation, 2.0, None);
                    }
                }

                // 绘制节点
                for (_, node) in snapshot.nodes.iter() {
                    self.draw_node(painter, node, 2.0);
                }
            }

            // 处理鼠标悬停事件
            // 查找鼠标位置是否在节点区域或者边区域，若是则设置悬停的节点或边
            if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                self.process_hover(pos);
            }

            // 检测点击事件：
            // 若双击位置落在某个节点区域，则进入编辑状态
            // 若单击位置落在某个节点区域或边区域，则选中节点或边
            if ui.input(|i| i.pointer.primary_clicked()) {
                if let Some(click_pos) = ui.input(|i| i.pointer.interact_pos()) {
                    self.process_click(click_pos);
                }
            }

            // 检测拖动事件，包括鼠标点击与抬起
            // 若鼠标左键按下，则开始拖动节点或者绘制边
            if ui.input(|i| i.pointer.primary_down()) && self.editing_node.is_none() {
                if self.dragging_node.is_none() && self.edge_start_node.is_none() {
                    if let Some(click_pos) = ui.input(|i| i.pointer.interact_pos()) {
                        // 判断点击的节点
                        let mut clicked_node = None;
                        let snapshot = self.graph.current_snapshot();
                        for (_, node) in snapshot.nodes.iter() {
                            let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                            let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                            let rect = Rect::from_center_size(node_pos, size);
                            if rect.contains(click_pos) {
                                clicked_node = Some(node);
                                break;
                            }
                        }

                        if let Some(node) = clicked_node {
                            let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);

                            if node_pos.distance(click_pos) < 4.0 {
                                // 如果节点中心和 click_pos 接近，则开始绘制边
                                self.edge_start_node = Some(node.id);
                                self.dragging_offset = Vec2::ZERO;
                            } else {
                                // 否则拖动节点
                                self.dragging_node = Some(node.id);
                            }
                        }
                    }
                }
                if self.dragging_node.is_some() {
                    // 获取鼠标拖动的位移
                    let drag_delta = ui.input(|i| i.pointer.delta());
                    self.dragging_offset += drag_delta;
                }
            }

            // 若鼠标左键抬起，则停止拖动节点
            if ui.input(|i| i.pointer.primary_released()) {
                // 如果设置拖拽节点
                if let Some(dragging_node) = self.dragging_node {
                    if let Some(node) = self.graph.current_snapshot().nodes.get(&dragging_node) {
                        let new_pos = Pos2::new(
                            node.coor.0 as f32 + self.dragging_offset.x,
                            node.coor.1 as f32 + self.dragging_offset.y,
                        );
                        self.graph
                            .update_entity_position(
                                dragging_node,
                                (new_pos.x as f64, new_pos.y as f64),
                            )
                            .unwrap();
                    }
                    // 重置变量
                    self.dragging_node = None;
                    self.dragging_offset = Vec2::ZERO;
                }

                // 如果设置绘制边
                if let Some(edge_start_node) = self.edge_start_node {
                    if self.edge_end_node.is_none()
                        && self
                            .graph
                            .current_snapshot()
                            .nodes
                            .get(&edge_start_node)
                            .is_some()
                    {
                        let snapshot = self.graph.current_snapshot();
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            for (id, node) in snapshot.nodes.iter() {
                                let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                                let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                                let rect = Rect::from_center_size(node_pos, size);
                                if rect.contains(pos) {
                                    self.edge_end_node = Some(*id);
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            // 检测删除事件：若按下 Delete 键，则删除选中的节点或边
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                if let Some(selected_node) = self.selected_node {
                    self.graph.remove_entity(selected_node).unwrap();
                    self.selected_node = None;
                } else if let Some((from, to)) = self.selected_edge {
                    self.graph.remove_edge(from, to).unwrap();
                    self.selected_edge = None;
                }
            }

            // 如果处于编辑状态，则弹出编辑窗口
            if let Some(edit_id) = self.editing_node {
                self.show_edit_window(edit_id, ctx);
            }

            // 如果选中了节点，则突出显示
            if let Some(selected_node) = self.selected_node {
                // 只在未拖动节点且未进入编辑时绘制
                if self.dragging_node.is_none() && self.editing_node.is_none() {
                    let snapshot = self.graph.current_snapshot();
                    if let Some(node) = snapshot.nodes.get(&selected_node) {
                        let pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                        let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                        let rect = Rect::from_center_size(pos, size);
                        let corner_radius = 10.0;

                        // 绘制边框
                        painter.rect_stroke(
                            rect,
                            corner_radius,
                            Stroke::new(4.0, Color32::from_rgb(54, 131, 248)),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }

            // 如果选中了边，则突出显示
            if let Some((from, to)) = self.selected_edge {
                // 只在未拖动节点且未进入编辑时绘制
                if self.dragging_node.is_none()
                    && self.editing_node.is_none()
                    && self.edge_start_node.is_none()
                    && self.edge_end_node.is_none()
                {
                    let snapshot = self.graph.current_snapshot();
                    if let (Some(from_node), Some(to_node)) =
                        (snapshot.nodes.get(&from), snapshot.nodes.get(&to))
                    {
                        if let Some(relation) = snapshot.edges.get(&(from, to)) {
                            // 绘制边
                            self.draw_edge(painter, from_node, to_node, *relation, 4.0, None);

                            // 绘制边连接的节点
                            for node in [from_node, to_node] {
                                self.draw_node(painter, node, 2.0);
                            }
                        }
                    }
                }
            }

            // 如果正在拖动节点，则进行绘制
            if let Some(dragging_node) = self.dragging_node {
                if self.editing_node.is_none()
                    && self.edge_start_node.is_none()
                    && self.edge_end_node.is_none()
                {
                    if let Some(node) = self.graph.current_snapshot().nodes.get(&dragging_node) {
                        let pos = Pos2::new(
                            node.coor.0 as f32 + self.dragging_offset.x,
                            node.coor.1 as f32 + self.dragging_offset.y,
                        );
                        let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                        let rect = Rect::from_center_size(pos, size);
                        let corner_radius = 10.0;

                        // 绘制填充矩形
                        let mut color = node.distinct_type.rect_color();
                        color[3] = 200; // 设置透明度
                        painter.rect_filled(rect, corner_radius, color);

                        // 绘制边框
                        painter.rect_stroke(
                            rect,
                            corner_radius,
                            Stroke::new(2.0, Color32::from_rgb(54, 131, 248)),
                            egui::StrokeKind::Outside,
                        );
                    }
                }
            }

            // 如果鼠标悬停在节点或边上，则进行绘制
            if let Some((hovered_node, is_center_hovered)) = self.hovered_node {
                if self.dragging_node.is_none() && self.editing_node.is_none() {
                    let snapshot = self.graph.current_snapshot();
                    if let Some(node) = snapshot.nodes.get(&hovered_node) {
                        let pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                        let size = Vec2::new(NODE_SIZE.x + 2.0, NODE_SIZE.y + 2.0);
                        let rect = Rect::from_center_size(pos, size);
                        let corner_radius = 10.0;

                        // 绘制边框
                        painter.rect_stroke(
                            rect,
                            corner_radius,
                            Stroke::new(4.0, Color32::from_gray(200)),
                            egui::StrokeKind::Outside,
                        );

                        // 绘制中心点
                        if self.edge_start_node.is_none() && self.edge_end_node.is_none() {
                            if is_center_hovered {
                                painter.circle(
                                    pos,
                                    4.0,
                                    Color32::WHITE,
                                    Stroke::new(4.0, Color32::GRAY),
                                );
                            } else {
                                painter.circle(
                                    pos,
                                    4.0,
                                    Color32::WHITE,
                                    Stroke::new(2.0, Color32::GRAY),
                                );
                            }
                        }
                    }
                }
            }

            // 如果鼠标悬停在边上，则进行绘制
            if let Some((from, to)) = self.hovered_edge {
                if self.dragging_node.is_none()
                    && self.editing_node.is_none()
                    && self.edge_start_node.is_none()
                    && self.edge_end_node.is_none()
                {
                    if let Some(selected_edge) = self.selected_edge {
                        if selected_edge == (from, to) {
                            return; // 选中的边已经突出显示，不需要再次绘制
                        }
                    }
                    let snapshot = self.graph.current_snapshot();
                    if let (Some(from_node), Some(to_node)) =
                        (snapshot.nodes.get(&from), snapshot.nodes.get(&to))
                    {
                        if let Some(relation) = snapshot.edges.get(&(from, to)) {
                            // 绘制边
                            self.draw_edge(
                                painter,
                                from_node,
                                to_node,
                                *relation,
                                4.0,
                                Some(Color32::from_gray(200)),
                            );

                            // 绘制边连接的节点
                            for node in [from_node, to_node] {
                                self.draw_node(painter, node, 2.0);
                            }
                        }
                    }
                }
            }

            // 如果正在绘制边，则进行绘制
            if let Some(edge_start_node) = self.edge_start_node {
                if let Some(edge_end_node) = self.edge_end_node {
                    if edge_start_node != edge_end_node {
                        self.show_relation_window(ctx, edge_start_node, edge_end_node);
                    }
                } else {
                    // 绘制正在绘制的边
                    let snapshot = self.graph.current_snapshot();
                    if let Some(from_node) = snapshot.nodes.get(&edge_start_node) {
                        let start = Pos2::new(from_node.coor.0 as f32, from_node.coor.1 as f32);
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            painter.line_segment([start, pos], Stroke::new(2.0, Color32::BLACK));
                        }
                    }
                }
            }
        });
    }
}

impl GraphApp {
    fn draw_edge(
        &self,
        painter: &Painter,
        from: &EntityNode,
        to: &EntityNode,
        relation: Relation,
        stroke_size: f32,
        color: Option<Color32>,
    ) {
        let start = Pos2::new(from.coor.0 as f32, from.coor.1 as f32);
        let end = Pos2::new(to.coor.0 as f32, to.coor.1 as f32);
        let mid = Pos2::new(start.x * 0.45 + end.x * 0.55, start.y * 0.45 + end.y * 0.55);
        let stroke = Stroke::new(stroke_size, color.unwrap_or(relation.arrow_color()));
        // 绘制起点到中点箭头
        let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
        let vec = end - mid;
        let tip_length = 8.0;
        let dir = vec.normalized();
        painter.line_segment([start, mid], stroke);
        painter.line_segment([mid, mid - tip_length * (rot * dir)], stroke);
        painter.line_segment([mid, mid - tip_length * (rot.inverse() * dir)], stroke);
        // 绘制中点到终点的线段
        painter.line_segment([mid, end], stroke);
    }

    fn draw_node(&self, painter: &Painter, node: &EntityNode, stroke_size: f32) {
        let pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
        let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
        let rect = Rect::from_center_size(pos, size);
        let corner_radius = 10.0;

        // 绘制填充矩形
        painter.rect_filled(rect, corner_radius, node.distinct_type.rect_color());

        // 绘制边框
        painter.rect_stroke(
            rect,
            corner_radius,
            Stroke::new(stroke_size, Color32::from_rgb(54, 131, 248)),
            egui::StrokeKind::Outside,
        );

        // 绘制节点类型
        let type_galley = painter.layout(
            node.distinct_type.class_name_abbr().to_string(),
            FontId::new(10.0, FontFamily::Proportional),
            Color32::from_rgb(189, 53, 61),
            rect.width() / 2.0,
        );
        let padding = Vec2::new(2.0, 2.0);
        let text_size = type_galley.size();
        let bg_size = text_size + 2.0 * padding;
        let gap = Vec2::new(4.0, 4.0);
        let bg_min = rect.min + gap;
        let bg_rect = Rect::from_min_size(bg_min, bg_size);
        painter.rect_filled(bg_rect, 3.0, Color32::from_rgb(255, 192, 122));
        let text_pos = bg_rect.min + padding;
        painter.galley(text_pos, type_galley, Color32::PLACEHOLDER);

        // 绘制节点附加类型
        let mut addon_types = node
            .addon_types
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>();
        addon_types.sort();
        let addon_types_str = addon_types.join(" ");
        let addon_font = FontId::new(8.0, FontFamily::Proportional);
        let addon_galley = painter.layout(
            addon_types_str.clone(),
            addon_font,
            Color32::from_rgb(189, 53, 61),
            rect.width() / 2.0,
        );
        let padding = Vec2::new(2.0, 2.0);
        let text_size = addon_galley.size();
        let bg_size = text_size + 2.0 * padding;
        let gap = Vec2::new(4.0, 4.0);
        let bg_min = rect.max - bg_size - gap;
        let bg_rect = Rect::from_min_size(bg_min, bg_size);
        painter.rect_filled(bg_rect, 3.0, Color32::from_rgb(255, 192, 122));
        let text_pos = bg_rect.min + padding;
        painter.galley(text_pos, addon_galley, Color32::PLACEHOLDER);

        // 绘制节点内容，使用默认字体
        let galley = painter.layout(
            node.content.clone(),
            FontId::new(12.0, FontFamily::Proportional),
            Color32::BLACK,
            size.x - 2.0 * corner_radius,
        );
        let text_pos = Pos2::new(pos.x - galley.size().x / 2.0, pos.y - galley.size().y / 2.0);
        painter.galley(text_pos, galley, Color32::PLACEHOLDER);
    }

    fn commit_edit(&mut self, edit_id: u64) -> Result<(), GraphError> {
        let addon_types = self
            .editing_addon_types
            .iter()
            .filter_map(|(t, selected)| if *selected { Some(*t) } else { None })
            .collect::<Vec<_>>();
        // 这里假设 snapshot 内部数据已不会导致借用冲突（可重新获取相关数据）
        self.graph.update_entity_content(
            edit_id,
            self.editing_content.clone(),
            self.editing_distinct_type,
            &addon_types,
        )?;
        self.editing_node = None;

        Ok(())
    }

    fn show_relation_window(&mut self, ctx: &Context, edge_start_node: u64, edge_end_node: u64) {
        egui::Window::new("设置关系")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("选择关系类型:");
                ui.vertical(|ui| {
                    ui.radio_value(&mut self.current_relation, Relation::Contain, "包含");
                    ui.radio_value(&mut self.current_relation, Relation::Order, "顺序");
                });

                ui.horizontal(|ui| {
                    if ui.button("确定").clicked() {
                        self.graph
                            .add_edge(edge_start_node, edge_end_node, self.current_relation)
                            .unwrap();
                        self.edge_start_node = None;
                        self.edge_end_node = None;
                    }
                    if ui.button("取消").clicked() {
                        self.edge_start_node = None;
                        self.edge_end_node = None;
                    }
                });
            });
    }

    fn show_edit_window(&mut self, edit_id: u64, ctx: &Context) {
        egui::Window::new("编辑节点内容")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                // 检查 Esc 键：退出编辑状态
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.editing_node = None;
                    return; // 退出窗口显示逻辑
                }
                // 检查 Ctrl + Enter 键：提交保存操作
                if ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
                    self.commit_edit(edit_id).unwrap();
                    return;
                }

                ui.label("修改节点类型:");
                ui.horizontal(|ui| {
                    ui.radio_value(
                        &mut self.editing_distinct_type,
                        DistinctEntityType::KnowledgeArena,
                        "知识领域",
                    );
                    ui.radio_value(
                        &mut self.editing_distinct_type,
                        DistinctEntityType::KnowledgePoint,
                        "知识点",
                    );
                    ui.radio_value(
                        &mut self.editing_distinct_type,
                        DistinctEntityType::KnowledgeDetail,
                        "知识细节",
                    );
                    ui.radio_value(
                        &mut self.editing_distinct_type,
                        DistinctEntityType::KnowledgeUnit,
                        "知识单元",
                    );
                });

                ui.separator();
                ui.label("修改节点附加类型:");
                ui.horizontal(|ui| {
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Example)
                            .or_default(),
                        "示例",
                    );
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Question)
                            .or_default(),
                        "问题",
                    );
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Practice)
                            .or_default(),
                        "练习",
                    );
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Thinking)
                            .or_default(),
                        "思考",
                    );
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Knowledge)
                            .or_default(),
                        "知识",
                    );
                    ui.checkbox(
                        self.editing_addon_types
                            .entry(AddonEntityType::Political)
                            .or_default(),
                        "思政",
                    );
                });

                ui.separator();
                ui.label("修改节点内容:");
                ui.text_edit_multiline(&mut self.editing_content);

                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        self.commit_edit(edit_id).unwrap();
                    }
                    if ui.button("取消").clicked() {
                        self.editing_node = None;
                    }
                });
            });
    }

    fn process_click(&mut self, pos: Pos2) {
        let now = time::Instant::now();
        let snapshot = self.graph.current_snapshot();

        let time_diff = now - self.last_click_time;
        let pos_diff = pos - self.last_click_pos;

        if time_diff < time::Duration::from_millis(300) && pos_diff.length() < 5.0 {
            // 认为是双击事件，查找点击位置是否在节点区域，若是则进入编辑节点状态
            for (id, node) in snapshot.nodes.iter() {
                let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                let rect = Rect::from_center_size(node_pos, size);
                if rect.contains(pos) {
                    self.editing_distinct_type = node.distinct_type;
                    self.editing_content = node.content.clone();
                    for t in node.addon_types.iter() {
                        self.editing_addon_types.insert(*t, true);
                    }
                    self.editing_node = Some(*id);
                    break;
                }
            }
        } else {
            // 认为是单击事件，查找点击位置是否在节点区域或者边区域，若是则选中节点或边
            // 重置选中状态
            self.selected_node = None;
            self.selected_edge = None;

            // 优先选中节点
            let snapshot = self.graph.current_snapshot();

            for (id, node) in snapshot.nodes.iter() {
                let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
                let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
                let rect = Rect::from_center_size(node_pos, size);
                if rect.contains(pos) {
                    self.selected_node = Some(*id);
                    break;
                }
            }

            // 若未选中节点，则尝试选中边
            if self.selected_node.is_none() {
                for ((from, to), _) in snapshot.edges.iter() {
                    if let (Some(from_node), Some(to_node)) =
                        (snapshot.nodes.get(from), snapshot.nodes.get(to))
                    {
                        let start = Pos2::new(from_node.coor.0 as f32, from_node.coor.1 as f32);
                        let end = Pos2::new(to_node.coor.0 as f32, to_node.coor.1 as f32);
                        // 计算点击位置到线段的距离
                        let dist = distance_point_to_segment(pos, start, end);
                        if dist < 5.0 {
                            self.selected_edge = Some((*from, *to));
                            break;
                        }
                    }
                }
            }
        }

        self.last_click_time = now;
        self.last_click_pos = pos;
    }

    fn process_hover(&mut self, pos: Pos2) {
        let snapshot = self.graph.current_snapshot();

        // 重置悬停状态
        self.hovered_node = None;
        self.hovered_edge = None;

        // 优先悬停节点
        for (id, node) in snapshot.nodes.iter() {
            let node_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
            let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y);
            let rect = Rect::from_center_size(node_pos, size);
            if rect.contains(pos) {
                if node_pos.distance(pos) < 4.0 {
                    self.hovered_node = Some((*id, true));
                } else {
                    self.hovered_node = Some((*id, false));
                }
                break;
            }
        }

        // 若未悬停节点，则尝试悬停边
        if self.hovered_node.is_none() {
            for ((from, to), _) in snapshot.edges.iter() {
                if let (Some(from_node), Some(to_node)) =
                    (snapshot.nodes.get(from), snapshot.nodes.get(to))
                {
                    let start = Pos2::new(from_node.coor.0 as f32, from_node.coor.1 as f32);
                    let end = Pos2::new(to_node.coor.0 as f32, to_node.coor.1 as f32);
                    // 计算点击位置到线段的距离
                    let dist = distance_point_to_segment(pos, start, end);
                    if dist < 5.0 {
                        self.hovered_edge = Some((*from, *to));
                        break;
                    }
                }
            }
        }
    }
}

impl DistinctEntityType {
    fn rect_color(&self) -> Color32 {
        match *self {
            DistinctEntityType::KnowledgeArena => Color32::from_rgb(255, 105, 97),
            DistinctEntityType::KnowledgePoint => Color32::from_rgb(189, 181, 225),
            DistinctEntityType::KnowledgeDetail => Color32::from_rgb(182, 215, 232),
            DistinctEntityType::KnowledgeUnit => Color32::from_rgb(176, 217, 128),
        }
    }

    fn class_name_abbr(&self) -> &str {
        match *self {
            DistinctEntityType::KnowledgeArena => "知识领域",
            DistinctEntityType::KnowledgePoint => "知识点",
            DistinctEntityType::KnowledgeDetail => "知识细节",
            DistinctEntityType::KnowledgeUnit => "知识单元",
        }
    }
}

impl AddonEntityType {
    fn name(&self) -> &str {
        match *self {
            AddonEntityType::Example => "示例",
            AddonEntityType::Question => "问题",
            AddonEntityType::Practice => "练习",
            AddonEntityType::Thinking => "思考",
            AddonEntityType::Knowledge => "知识",
            AddonEntityType::Political => "思政",
        }
    }
}

impl Relation {
    fn arrow_color(&self) -> Color32 {
        match *self {
            Relation::Contain => Color32::BLACK,
            Relation::Order => Color32::BLUE,
        }
    }
}

fn distance_point_to_segment(point: Pos2, start: Pos2, end: Pos2) -> f32 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;

    if dx == 0.0 && dy == 0.0 {
        // 线段是一个点
        return point.distance(start);
    }

    let t = ((point.x - start.x) * dx + (point.y - start.y) * dy) / (dx * dx + dy * dy);

    if t <= 0.0 {
        // 投影点在线段起点之前
        point.distance(start)
    } else if t >= 1.0 {
        // 投影点在线段终点之后
        point.distance(end)
    } else {
        // 投影点在线段上
        let projection = Pos2 {
            x: start.x + t * dx,
            y: start.y + t * dy,
        };
        point.distance(projection)
    }
}
