use std::{collections::HashMap, time};

use eframe::{
    App,
    egui::{self, Align2, Color32, Context, FontFamily, FontId, Painter, Pos2, Rect, Stroke, Vec2},
    emath::Rot2,
};

use crate::{
    error::GraphError,
    file::FiledKnowledgeGraph,
    graph::{AddonEntityType, DistinctEntityType, EntityNode, Relation},
};

const NODE_SIZE: Vec2 = Vec2::new(150.0, 120.0);
const TOP_PANEL_HEIGHT: f32 = 50.0;

pub struct GraphApp {
    pub graph: Option<FiledKnowledgeGraph>,

    // 上一次单机左键的信息
    last_click_time: time::Instant,
    last_click_pos: Pos2,

    // 编辑的节点
    editing_node: Option<u64>,
    editing_content: String,
    editing_distinct_type: DistinctEntityType,
    editing_addon_types: HashMap<AddonEntityType, bool>,
    editing_new_node: bool,

    // 编辑的边
    editing_edge: Option<(u64, u64)>,
    editing_relation: Relation,

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

    // 错误信息 (title, message)
    error: Option<(String, String)>,
    info: (String, time::Instant),

    // 用于记录图谱整体平移的偏移量
    scroll_offset: Vec2,

    // 用于记录缩放比例和缩放中心
    zoom_factor: f32,
}

impl Default for GraphApp {
    fn default() -> Self {
        Self {
            graph: None,
            last_click_pos: Pos2::new(-100.0, -100.0), // 初始化为一个不可能的位置
            last_click_time: time::Instant::now() - time::Duration::from_secs(1), // 初始化为一个不可能的时间
            editing_node: None,
            editing_content: String::new(),
            editing_distinct_type: DistinctEntityType::KnowledgeArena,
            editing_addon_types: HashMap::with_capacity(6),
            editing_new_node: false,
            editing_edge: None,
            editing_relation: Relation::Contain,
            selected_node: None,
            selected_edge: None,
            dragging_node: None,
            dragging_offset: Vec2::ZERO,
            hovered_node: None,
            hovered_edge: None,
            edge_start_node: None,
            edge_end_node: None,
            current_relation: Relation::Contain,
            error: None,
            info: (
                String::new(),
                time::Instant::now() - time::Duration::from_secs(3),
            ),
            scroll_offset: Vec2::ZERO,
            zoom_factor: 1.0,
        }
    }
}

macro_rules! dialog_error {
    ($this:ident, $result:expr, $ignored_errors:expr, $msg:expr) => {
        if let Err(e) = $result {
            if $ignored_errors.iter().all(|err| e != *err) {
                $this.error = Some(($msg.to_string(), e.to_string()));
            }
        }
    };
}

impl App for GraphApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("控制栏")
            .min_height(TOP_PANEL_HEIGHT)
            .max_height(TOP_PANEL_HEIGHT)
            .show(ctx, |ui| {
                self.show_topbar(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            // 绘制错误信息
            self.show_error_popup(ctx);

            // 未打开文件时，显示提示信息
            if self.graph.is_none() {
                self.show_welcome_page(ui);
                return;
            }

            let scroll_area = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .drag_to_scroll(false); // 禁用拖动滚动，避免与拖动节点冲突

            let scroll_response = scroll_area.show(ui, |ui| {
                // 计算内容边界以正确显示滚动条
                let mut content_rect = Rect::NOTHING;
                if let Some(graph) = self.graph.as_ref() {
                    let snapshot = graph.current_snapshot();
                    for node in snapshot.nodes.values() {
                        let pos = self.node_screen_pos(node);
                        let node_rect = Rect::from_center_size(pos, NODE_SIZE * self.zoom_factor);
                        content_rect = content_rect.union(node_rect);
                    }
                }
                ui.expand_to_include_rect(content_rect); // 告诉UI内容区域大小

                // 原有绘制和处理逻辑
                let painter = ui.painter();

                self.draw_edges_and_nodes(painter);

                // 如果选中了节点，则突出显示
                self.show_selected_node(painter);

                // 如果选中了边，则突出显示
                self.show_selected_edge(painter);

                // 如果正在拖动节点，则进行绘制
                self.show_dragging_node(painter);

                // 如果鼠标悬停在节点或边上，则进行绘制
                self.show_hovered_node(painter);

                // 如果鼠标悬停在边上，则进行绘制
                self.show_hovered_edge(painter);

                // 如果正在绘制边，则进行绘制
                self.show_drawing_edge(ui, painter, ctx);
            });

            self.scroll_offset = scroll_response.state.offset;

            // 处理鼠标悬停事件
            self.process_hover(ui);

            // 检测点击事件
            self.process_primary_click(ui);

            // 检测拖动事件，包括鼠标点击与抬起
            self.process_primary_down(ui);

            // 若鼠标左键抬起，则停止拖动节点
            self.process_primary_up(ui);

            // 检测删除
            self.process_keyboard_delete(ui);

            // 检测撤销和恢复
            self.process_undo_redo(ui);

            // 处理缩放
            self.process_zoom(ctx);

            // 检测保存按键
            self.process_keyboard_save(ui);

            // 如果处于节点编辑状态，则弹出编辑窗口
            self.show_node_edit_window(ctx);

            // 如果处于边编辑状态，则弹出编辑窗口
            self.show_edge_edit_window(ctx);
        });
    }
}

impl GraphApp {
    #[inline]
    fn is_editing(&self) -> bool {
        self.editing_node.is_some() || self.editing_edge.is_some()
    }

    #[inline]
    fn is_linking_edge(&self) -> bool {
        self.edge_start_node.is_some() || self.edge_end_node.is_some()
    }

    #[inline]
    fn is_dragging(&self) -> bool {
        self.dragging_node.is_some()
    }

    #[inline]
    fn node_screen_pos(&self, node: &EntityNode) -> Pos2 {
        let content_pos = Pos2::new(node.coor.0 as f32, node.coor.1 as f32);
        (content_pos * self.zoom_factor) - self.scroll_offset + Vec2::new(0.0, TOP_PANEL_HEIGHT)
    }

    #[inline]
    fn screen_to_content(&self, screen_pos: Pos2) -> Pos2 {
        (screen_pos - Vec2::new(0.0, TOP_PANEL_HEIGHT) + self.scroll_offset) / self.zoom_factor
    }

    fn draw_edges_and_nodes(&self, painter: &Painter) {
        if let Some(graph) = self.graph.as_ref() {
            // 从图谱中获取当前快照
            let snapshot = graph.current_snapshot();

            // 先绘制边
            for ((from, to), relation) in snapshot.edges.iter() {
                if let (Some(from_node), Some(to_node)) =
                    (snapshot.nodes.get(from), snapshot.nodes.get(to))
                {
                    self.draw_edge(painter, from_node, to_node, *relation, 2.0, Color32::BLACK);
                }
            }

            // 绘制节点
            for (_, node) in snapshot.nodes.iter() {
                self.draw_node(painter, node, 2.0);
            }
        }
    }

    fn draw_edge(
        &self,
        painter: &Painter,
        from: &EntityNode,
        to: &EntityNode,
        relation: Relation,
        stroke_size: f32,
        color: Color32,
    ) {
        let start = self.node_screen_pos(from);
        let end = self.node_screen_pos(to);
        let stroke = Stroke::new(stroke_size * self.zoom_factor, color);
        painter.line_segment([start, end], stroke);
        let tip_length = 8.0;
        match relation {
            Relation::Order => {
                // 绘制箭头
                let mid = Pos2::new(start.x * 0.45 + end.x * 0.55, start.y * 0.45 + end.y * 0.55);
                let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
                let vec = end - mid;
                let dir = vec.normalized();
                painter.line_segment([mid, mid - tip_length * (rot * dir)], stroke);
                painter.line_segment([mid, mid - tip_length * (rot.inverse() * dir)], stroke);
            }
            Relation::Contain => {
                // 绘制半圆
                // 以边中点作为半圆中心，半径可以根据需要调整（这里使用 tip_length 作为半径示例）
                let radius = tip_length;
                // 计算边的方向角
                let line_angle = (end - start).angle();
                // 设定起始角度，使半圆向上凸出（相对于线段方向）
                let start_angle = line_angle - std::f32::consts::FRAC_PI_2;
                let end_angle = start_angle + std::f32::consts::PI;
                let steps = 20; // 分段数，可调节平滑程度
                let mut arc_points = Vec::with_capacity(steps + 1);
                let mid = Pos2::new(start.x * 0.5 + end.x * 0.5, start.y * 0.5 + end.y * 0.5);
                for i in 0..=steps {
                    let a = start_angle + (end_angle - start_angle) * (i as f32 / steps as f32);
                    // 使用 mid 作为圆弧中心
                    let p = mid + Vec2::new(a.cos(), a.sin()) * radius;
                    arc_points.push(p);
                }
                painter.add(egui::Shape::line(arc_points, stroke));
            }
        }
    }

    fn draw_node(&self, painter: &Painter, node: &EntityNode, stroke_size: f32) {
        let pos = self.node_screen_pos(node);
        let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
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
            FontId::new(10.0 * self.zoom_factor, FontFamily::Proportional),
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
        if !addon_types.is_empty() {
            addon_types.sort();
            let addon_types_str = addon_types.join(" ");
            let addon_font = FontId::new(8.0 * self.zoom_factor, FontFamily::Proportional);
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
        }

        // 绘制节点内容，使用默认字体
        let galley = painter.layout(
            node.content.clone(),
            FontId::new(12.0 * self.zoom_factor, FontFamily::Proportional),
            Color32::BLACK,
            size.x - 2.0 * corner_radius,
        );
        let text_pos = Pos2::new(pos.x - galley.size().x / 2.0, pos.y - galley.size().y / 2.0);
        painter.galley(text_pos, galley, Color32::PLACEHOLDER);
    }

    fn commit_edit(&mut self, edit_id: u64) -> Result<(), GraphError> {
        if let Some(graph) = self.graph.as_mut() {
            let addon_types = self
                .editing_addon_types
                .iter()
                .filter_map(|(t, selected)| if *selected { Some(*t) } else { None })
                .collect::<Vec<_>>();
            // 这里假设 snapshot 内部数据已不会导致借用冲突（可重新获取相关数据）
            graph.update_entity_content(
                edit_id,
                self.editing_content.clone(),
                self.editing_distinct_type,
                &addon_types,
            )?;
            self.editing_node = None;
        }
        Ok(())
    }

    fn show_welcome_page(&mut self, ui: &mut egui::Ui) {
        if self.graph.is_none() {
            ui.with_layout(
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.label(
                        egui::RichText::new("请点击上方按钮新建或添加文件")
                            .color(egui::Color32::GRAY)
                            .size(20.0),
                    );
                },
            );
        }
    }

    fn show_relation_window(&mut self, ctx: &Context, edge_start_node: u64, edge_end_node: u64) {
        if self.graph.is_none() {
            return;
        }

        egui::Window::new("设置关系")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("选择关系类型:");
                ui.vertical(|ui| {
                    ui.radio_value(&mut self.current_relation, Relation::Contain, "包含");
                    ui.radio_value(&mut self.current_relation, Relation::Order, "顺序");
                });

                ui.horizontal(|ui| {
                    if ui.button("确定").clicked() {
                        dialog_error!(
                            self,
                            self.graph.as_mut().unwrap().add_edge(
                                edge_start_node,
                                edge_end_node,
                                self.current_relation
                            ),
                            &[],
                            "添加边失败"
                        );
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

    fn show_edge_edit_window(&mut self, ctx: &Context) {
        if self.graph.is_some() {
            if let Some((from_id, to_id)) = self.editing_edge {
                egui::Window::new("编辑边关系")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        ui.label("修改边关系类型:");
                        ui.vertical(|ui| {
                            ui.radio_value(&mut self.editing_relation, Relation::Contain, "包含");
                            ui.radio_value(&mut self.editing_relation, Relation::Order, "顺序");
                        });

                        ui.horizontal(|ui| {
                            if ui.button("保存").clicked() {
                                dialog_error!(
                                    self,
                                    self.graph.as_mut().unwrap().update_edge(
                                        from_id,
                                        to_id,
                                        self.editing_relation
                                    ),
                                    &[],
                                    "更新边失败"
                                );
                                self.editing_edge = None;
                            }
                            if ui.button("取消").clicked() {
                                self.editing_edge = None;
                            }
                        });
                    });
            }
        }
    }

    fn show_node_edit_window(&mut self, ctx: &Context) {
        if self.graph.is_some() {
            if let Some(edit_id) = self.editing_node {
                egui::Window::new("编辑节点内容")
                    .collapsible(false)
                    .resizable(false)
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .show(ctx, |ui| {
                        // 检查 Esc 键：退出编辑状态
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.editing_node = None;
                            return; // 退出窗口显示逻辑
                        }
                        // 检查 Ctrl + Enter 键：提交保存操作
                        if ui.input(|i| i.key_pressed(egui::Key::Enter) && i.modifiers.command) {
                            dialog_error!(self, self.commit_edit(edit_id), &[], "保存节点失败");
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
                                dialog_error!(self, self.commit_edit(edit_id), &[], "保存节点失败");
                            }
                            if ui.button("取消").clicked() {
                                // 如果是新建的节点，则删除
                                if self.editing_new_node {
                                    dialog_error!(
                                        self,
                                        self.graph.as_mut().unwrap().remove_entity(edit_id),
                                        &[],
                                        "删除节点失败"
                                    );
                                }
                                self.editing_node = None;
                            }
                        });
                    });
            }
        }
    }

    fn process_primary_click(&mut self, ui: &egui::Ui) {
        if self.graph.is_none() {
            return;
        }
        if ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(click_pos) = ui.input(|i| i.pointer.interact_pos()) {
                let now = time::Instant::now();
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();

                let time_diff = now - self.last_click_time;
                let pos_diff = click_pos - self.last_click_pos;

                if time_diff < time::Duration::from_millis(300) && pos_diff.length() < 5.0 {
                    // 认为是双击事件，查找点击位置是否在节点区域，若是则进入编辑节点状态
                    if self.editing_node.is_none() {
                        for (id, node) in snapshot.nodes.iter() {
                            let node_pos = self.node_screen_pos(node);
                            let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
                            let rect = Rect::from_center_size(node_pos, size);
                            if rect.contains(click_pos) {
                                self.editing_distinct_type = node.distinct_type;
                                self.editing_content = node.content.clone();
                                for t in node.addon_types.iter() {
                                    self.editing_addon_types.insert(*t, true);
                                }
                                self.editing_node = Some(*id);
                                self.editing_new_node = false;
                                break;
                            }
                        }
                    }

                    // 查找是否在边区域，若是则选中边
                    if self.editing_node.is_none() {
                        for ((from, to), _) in snapshot.edges.iter() {
                            if let (Some(from_node), Some(to_node)) =
                                (snapshot.nodes.get(from), snapshot.nodes.get(to))
                            {
                                let start = self.node_screen_pos(from_node);
                                let end = self.node_screen_pos(to_node);
                                // 计算点击位置到线段的距离
                                let dist = distance_point_to_segment(click_pos, start, end);
                                if dist < 5.0 {
                                    self.editing_edge = Some((*from, *to));
                                    break;
                                }
                            }
                        }
                    }

                    // 如果未选中节点，则认为是新创建一个节点
                    // 但是需要排除点击在顶部控制栏的情况
                    if !self.is_editing() && click_pos.y > TOP_PANEL_HEIGHT {
                        let node_pos = self.screen_to_content(click_pos);
                        let new_id = self.graph.as_mut().unwrap().add_entity(
                            String::new(),
                            DistinctEntityType::KnowledgePoint,
                            &[],
                            (node_pos.x as f64, node_pos.y as f64),
                        );
                        self.editing_distinct_type = DistinctEntityType::KnowledgePoint;
                        self.editing_content = String::new();
                        self.editing_addon_types.clear();
                        self.editing_node = Some(new_id);
                        self.editing_new_node = true;
                    }
                } else if !self.is_editing() {
                    // 认为是单击事件，查找点击位置是否在节点区域或者边区域，若是则选中节点或边
                    // 重置选中状态
                    self.selected_node = None;
                    self.selected_edge = None;

                    // 优先选中节点
                    let snapshot = self.graph.as_ref().unwrap().current_snapshot();

                    for (id, node) in snapshot.nodes.iter() {
                        let node_pos = self.node_screen_pos(node);
                        let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
                        let rect = Rect::from_center_size(node_pos, size);
                        if rect.contains(click_pos) {
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
                                let start = self.node_screen_pos(from_node);
                                let end = self.node_screen_pos(to_node);
                                // 计算点击位置到线段的距离
                                let dist = distance_point_to_segment(click_pos, start, end);
                                if dist < 5.0 {
                                    self.selected_edge = Some((*from, *to));
                                    break;
                                }
                            }
                        }
                    }
                }

                self.last_click_time = now;
                self.last_click_pos = click_pos;
            }
        }
    }

    fn process_hover(&mut self, ui: &egui::Ui) {
        if let Some(graph) = self.graph.as_ref() {
            if let Some(pos) = ui.input(|i| i.pointer.latest_pos()) {
                let snapshot = graph.current_snapshot();

                // 重置悬停状态
                self.hovered_node = None;
                self.hovered_edge = None;

                // 优先悬停节点
                for (id, node) in snapshot.nodes.iter() {
                    let node_pos = self.node_screen_pos(node);
                    let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
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
                            let start = self.node_screen_pos(from_node);
                            let end = self.node_screen_pos(to_node);
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
    }

    fn process_primary_down(&mut self, ui: &egui::Ui) {
        if let Some(graph) = self.graph.as_ref() {
            if ui.input(|i| i.pointer.primary_down()) && !self.is_editing() {
                if !self.is_dragging() && self.edge_start_node.is_none() {
                    if let Some(click_pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let window_size = ui.ctx().screen_rect();
                        if click_pos.y < TOP_PANEL_HEIGHT
                            || click_pos.y > window_size.height() - 40.0
                            || click_pos.x > window_size.width() - 40.0
                        {
                            return;
                        }
                        // 判断点击的节点
                        let mut clicked_node = None;
                        let snapshot = graph.current_snapshot();
                        for (_, node) in snapshot.nodes.iter() {
                            let node_pos = self.node_screen_pos(node);
                            let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
                            let rect = Rect::from_center_size(node_pos, size);
                            if rect.contains(click_pos) {
                                clicked_node = Some(node);
                                break;
                            }
                        }

                        if let Some(node) = clicked_node {
                            let node_pos = self.node_screen_pos(node);

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
                // 获取鼠标拖动的位移
                if self.is_dragging() {
                    let drag_delta = ui.input(|i| i.pointer.delta());
                    self.dragging_offset += drag_delta;
                }
            }
        }
    }

    fn process_primary_up(&mut self, ui: &egui::Ui) {
        if self.graph.is_none() {
            return;
        }
        if ui.input(|i| i.pointer.primary_released()) {
            // 如果设置拖拽节点
            if let Some(dragging_node) = self.dragging_node {
                if let Some(node) = self
                    .graph
                    .as_ref()
                    .unwrap()
                    .current_snapshot()
                    .nodes
                    .get(&dragging_node)
                {
                    let new_pos = Pos2::new(
                        node.coor.0 as f32 + self.dragging_offset.x,
                        node.coor.1 as f32 + self.dragging_offset.y,
                    );
                    dialog_error!(
                        self,
                        self.graph.as_mut().unwrap().update_entity_position(
                            dragging_node,
                            (new_pos.x as f64, new_pos.y as f64),
                        ),
                        &[],
                        "更新节点位置失败"
                    );
                }
                // 设置选中节点
                self.selected_node = self.dragging_node;

                // 重置变量
                self.dragging_node = None;
                self.dragging_offset = Vec2::ZERO;
            }

            // 如果设置绘制边
            if let Some(edge_start_node) = self.edge_start_node {
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
                if self.edge_end_node.is_none() && snapshot.nodes.get(&edge_start_node).is_some() {
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        for (id, node) in snapshot.nodes.iter() {
                            let node_pos = self.node_screen_pos(node);
                            let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
                            let rect = Rect::from_center_size(node_pos, size);
                            if rect.contains(pos) {
                                self.edge_end_node = Some(*id);
                                break;
                            }
                        }
                        // 如果未选中节点，则取消绘制边
                        if self.edge_end_node.is_none() {
                            self.edge_start_node = None;
                        }
                    }
                }
            }
        }
    }

    fn process_zoom(&mut self, ctx: &Context) {
        let zoom_delta = ctx.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > f32::EPSILON {
            if let Some(mouse_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                self.zoom_factor *= zoom_delta;
                self.zoom_factor = self.zoom_factor.clamp(0.5, 3.0);
            }
        }
    }

    fn process_keyboard_delete(&mut self, ui: &egui::Ui) {
        if let Some(graph) = self.graph.as_mut() {
            if ui.input(|i| i.key_pressed(egui::Key::Delete)) {
                if let Some(selected_node) = self.selected_node {
                    dialog_error!(
                        self,
                        graph.remove_entity(selected_node),
                        &[],
                        "删除节点失败"
                    );
                    self.selected_node = None;
                } else if let Some((from, to)) = self.selected_edge {
                    dialog_error!(self, graph.remove_edge(from, to), &[], "删除边失败");
                    self.selected_edge = None;
                }
            }
        }
    }

    fn process_keyboard_save(&mut self, ui: &egui::Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::S) && i.modifiers.command) {
            if let Some(graph) = self.graph.as_mut() {
                if let Err(e) = graph.save() {
                    self.error = Some((
                        format!(
                            "保存 {} 失败",
                            graph.file_path.as_os_str().to_string_lossy()
                        ),
                        e.to_string(),
                    ));
                }
                self.info = ("保存成功".to_string(), time::Instant::now());
            }
        }
    }

    fn process_undo_redo(&mut self, ui: &egui::Ui) {
        if self.graph.is_none() {
            return;
        }

        // 检测撤销
        if ui.input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.command)
            && !self.is_editing()
            && !self.is_linking_edge()
            && !self.is_dragging()
        {
            dialog_error!(
                self,
                self.graph.as_mut().unwrap().undo(),
                &[GraphError::NothingToUndo],
                "撤销失败"
            );
        }

        // 检测重做
        if ui.input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.command)
            && !self.is_editing()
            && !self.is_linking_edge()
            && !self.is_dragging()
        {
            dialog_error!(
                self,
                self.graph.as_mut().unwrap().redo(),
                &[GraphError::NothingToRedo],
                "恢复失败"
            );
        }
    }

    fn show_selected_node(&self, painter: &Painter) {
        if self.graph.is_none() {
            return;
        }

        if let Some(selected_node) = self.selected_node {
            // 只在未拖动节点且未进入编辑时绘制
            if !self.is_dragging() && !self.is_editing() {
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
                if let Some(node) = snapshot.nodes.get(&selected_node) {
                    let pos = self.node_screen_pos(node);
                    let size =
                        Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor + Vec2::splat(3.0);
                    let rect = Rect::from_center_size(pos, size);
                    let corner_radius = 10.0;

                    // 绘制边框
                    painter.rect_stroke(
                        rect,
                        corner_radius,
                        Stroke::new(6.0, Color32::RED),
                        egui::StrokeKind::Outside,
                    );
                }
            }
        }
    }

    fn show_selected_edge(&self, painter: &Painter) {
        if self.graph.is_none() {
            return;
        }

        if let Some((from, to)) = self.selected_edge {
            // 只在未拖动节点且未进入编辑时绘制
            if !self.is_dragging() && !self.is_editing() && !self.is_linking_edge() {
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
                if let (Some(from_node), Some(to_node)) =
                    (snapshot.nodes.get(&from), snapshot.nodes.get(&to))
                {
                    if let Some(relation) = snapshot.edges.get(&(from, to)) {
                        // 绘制边
                        self.draw_edge(painter, from_node, to_node, *relation, 6.0, Color32::RED);

                        // 绘制边连接的节点
                        for node in [from_node, to_node] {
                            self.draw_node(painter, node, 2.0);
                        }
                    }
                }
            }
        }
    }

    fn show_dragging_node(&self, painter: &Painter) {
        if self.graph.is_none() {
            return;
        }

        if let Some(dragging_node) = self.dragging_node {
            if !self.is_editing() && !self.is_linking_edge() {
                if let Some(node) = self
                    .graph
                    .as_ref()
                    .unwrap()
                    .current_snapshot()
                    .nodes
                    .get(&dragging_node)
                {
                    let pos = self.node_screen_pos(node) + self.dragging_offset;
                    let size = Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor;
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
    }

    fn show_hovered_node(&self, painter: &Painter) {
        if self.graph.is_none() {
            return;
        }

        if let Some((hovered_node, is_center_hovered)) = self.hovered_node {
            if !self.is_dragging() && !self.is_editing() {
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
                if let Some(node) = snapshot.nodes.get(&hovered_node) {
                    let pos = self.node_screen_pos(node);
                    let size =
                        Vec2::new(NODE_SIZE.x, NODE_SIZE.y) * self.zoom_factor + Vec2::splat(3.0);
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
                    if !self.is_linking_edge() {
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
    }

    fn show_hovered_edge(&self, painter: &Painter) {
        if self.graph.is_none() {
            return;
        }

        if let Some((from, to)) = self.hovered_edge {
            if !self.is_dragging() && !self.is_editing() && !self.is_linking_edge() {
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
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
                            Color32::from_gray(200),
                        );

                        // 绘制边连接的节点
                        for node in [from_node, to_node] {
                            self.draw_node(painter, node, 2.0);
                        }
                    }
                }
            }
        }
    }

    fn show_drawing_edge(&mut self, ui: &egui::Ui, painter: &Painter, ctx: &Context) {
        if self.graph.is_none() {
            return;
        }

        if let Some(edge_start_node) = self.edge_start_node {
            if let Some(edge_end_node) = self.edge_end_node {
                if edge_start_node != edge_end_node {
                    self.show_relation_window(ctx, edge_start_node, edge_end_node);
                } else {
                    self.edge_start_node = None;
                    self.edge_end_node = None;
                }
            } else {
                // 绘制正在绘制的边
                let snapshot = self.graph.as_ref().unwrap().current_snapshot();
                if let Some(from_node) = snapshot.nodes.get(&edge_start_node) {
                    let start = self.node_screen_pos(from_node);
                    if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                        painter.line_segment([start, pos], Stroke::new(2.0, Color32::BLACK));
                    }
                }
            }
        }
    }

    fn show_error_popup(&mut self, ctx: &Context) {
        if let Some((ref title, ref message)) = self.error.clone() {
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("确定").clicked() {
                        self.error = None;
                    }
                });
        }
    }

    fn show_topbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            let icon_size = Vec2::new(TOP_PANEL_HEIGHT * 0.7, TOP_PANEL_HEIGHT * 0.7);
            if ui
                .add_sized(
                    icon_size,
                    egui::ImageButton::new(egui::include_image!(
                        "../assets/note_add_35dp_5985E1_FILL0_wght400_GRAD0_opsz40.svg"
                    )),
                )
                .on_hover_text("新建文件")
                .clicked()
            {
                if let Some(file) = rfd::FileDialog::new()
                    .set_title("选择保存位置并输入文件名")
                    .add_filter("XML 文件", &["xml"])
                    .save_file()
                {
                    if let Some(graph) = self.graph.as_mut() {
                        if let Err(e) = graph.save() {
                            self.error = Some((
                                format!(
                                    "保存 {} 失败",
                                    graph.file_path.as_os_str().to_string_lossy()
                                ),
                                e.to_string(),
                            ));
                        }
                    }

                    match FiledKnowledgeGraph::new(&file, true) {
                        Ok(graph) => self.graph = Some(graph),
                        Err(e) => {
                            self.error = Some((
                                format!("打开 {} 失败", file.as_os_str().to_string_lossy()),
                                e.to_string(),
                            ))
                        }
                    }
                }
            }
            if ui
                .add_sized(
                    icon_size,
                    egui::ImageButton::new(egui::include_image!(
                        "../assets/file_open_35dp_5985E1_FILL0_wght400_GRAD0_opsz40.svg"
                    )),
                )
                .on_hover_text("打开文件")
                .clicked()
            {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("XML 文件", &["xml"])
                    .pick_file()
                {
                    if let Some(graph) = self.graph.as_mut() {
                        if let Err(e) = graph.save() {
                            self.error = Some((
                                format!(
                                    "保存 {} 失败",
                                    graph.file_path.as_os_str().to_string_lossy()
                                ),
                                e.to_string(),
                            ));
                        }
                    }
                    match FiledKnowledgeGraph::new(&file, false) {
                        Ok(graph) => self.graph = Some(graph),
                        Err(e) => {
                            self.error = Some((
                                format!("打开 {} 失败", file.as_os_str().to_string_lossy()),
                                e.to_string(),
                            ))
                        }
                    }
                }
            }
            if ui
                .add_sized(
                    icon_size,
                    egui::ImageButton::new(egui::include_image!(
                        "../assets/save_35dp_5985E1_FILL0_wght400_GRAD0_opsz40.svg"
                    )),
                )
                .on_hover_text("保存文件")
                .clicked()
            {
                if let Some(graph) = self.graph.as_mut() {
                    if let Err(e) = graph.save() {
                        self.error = Some((
                            format!(
                                "保存 {} 失败",
                                graph.file_path.as_os_str().to_string_lossy()
                            ),
                            e.to_string(),
                        ));
                    }
                    self.info = ("保存成功".to_string(), time::Instant::now());
                }
            }
            if ui
                .add_sized(
                    icon_size,
                    egui::ImageButton::new(egui::include_image!(
                        "../assets/arrow_back_35dp_5985E1_FILL0_wght400_GRAD0_opsz40.svg"
                    )),
                )
                .on_hover_text("撤销")
                .clicked()
            {
                if let Some(graph) = &mut self.graph {
                    dialog_error!(self, graph.undo(), &[GraphError::NothingToUndo], "撤销失败");
                }
            }
            if ui
                .add_sized(
                    icon_size,
                    egui::ImageButton::new(egui::include_image!(
                        "../assets/arrow_forward_35dp_5985E1_FILL0_wght400_GRAD0_opsz40.svg"
                    )),
                )
                .on_hover_text("恢复")
                .clicked()
            {
                if let Some(graph) = self.graph.as_mut() {
                    dialog_error!(self, graph.redo(), &[GraphError::NothingToRedo], "恢复失败");
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (info, last_update_time) = &self.info;
                if time::Instant::now() - *last_update_time < time::Duration::from_secs(1) {
                    ui.label(info);
                }
            });
        });
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
