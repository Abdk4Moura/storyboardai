use eframe::egui;
use eframe::wasm_bindgen::JsCast;
use egui::{
    Color32, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Vec2,
};
use std::collections::HashMap;
use std::sync::mpsc;
use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub enum NodeData {
    Concept {
        text: String,
    },
    YouComResearch {
        query: String,
        result: Option<String>,
        is_loading: bool,
    },
    Visual {
        prompt: String,
        #[serde(skip)]
        texture: Option<egui::TextureHandle>,
        is_loading: bool,
    },
    FoxitExport {
        status: String,
    },
}

impl fmt::Debug for NodeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Concept { text } => f.debug_struct("Concept").field("text", text).finish(),
            Self::YouComResearch { query, result, is_loading } => f.debug_struct("YouComResearch").field("query", query).field("result", result).field("is_loading", is_loading).finish(),
            Self::Visual { prompt, is_loading, .. } => f.debug_struct("Visual").field("prompt", prompt).field("is_loading", is_loading).finish(),
            Self::FoxitExport { status } => f.debug_struct("FoxitExport").field("status", status).finish(),
        }
    }
}

impl PartialEq for NodeData {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Concept { text: a }, Self::Concept { text: b }) => a == b,
            (Self::YouComResearch { query: a, result: b, is_loading: c }, Self::YouComResearch { query: x, result: y, is_loading: z }) => a == x && b == y && c == z,
            (Self::Visual { prompt: a, is_loading: b, .. }, Self::Visual { prompt: x, is_loading: y, .. }) => a == x && b == y,
            (Self::FoxitExport { status: a }, Self::FoxitExport { status: b }) => a == b,
            _ => false,
        }
    }
}

pub enum AppMessage {
    TextResponse(u64, String),
    ImageResponse(u64, Vec<u8>),
    Error(u64, String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: u64,
    pub position: Pos2,
    pub size: Vec2,
    pub data: NodeData,
    pub selected: bool,
    pub velocity: Vec2,
}

impl Node {
    pub fn new(id: u64, position: Pos2, data: NodeData) -> Self {
        Self {
            id,
            position,
            size: Vec2::new(250.0, 300.0),
            data,
            selected: false,
            velocity: Vec2::ZERO,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(self.position, self.size)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edge {
    pub id: u64,
    pub from: u64,
    pub to: u64,
}

pub struct CanvasState {
    pub nodes: HashMap<u64, Node>,
    pub edges: Vec<Edge>,
    pub next_id: u64,
    pub camera_offset: Vec2,
    pub camera_zoom: f32,
    pub dragging_node: Option<u64>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_id: 1,
            camera_offset: Vec2::ZERO,
            camera_zoom: 1.0,
            dragging_node: None,
        }
    }
}

pub struct StoryBoardApp {
    state: CanvasState,
    http_rx: mpsc::Receiver<AppMessage>,
    http_tx: mpsc::Sender<AppMessage>,
    frame_times: Vec<f32>,
}

impl StoryBoardApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (http_tx, http_rx) = mpsc::channel();
        let mut app = Self {
            state: CanvasState::default(),
            http_rx,
            http_tx,
            frame_times: Vec::new(),
        };
        app.setup_demo_scene();
        app
    }

    fn setup_demo_scene(&mut self) {
        let c1_id = self.add_node(
            Pos2::new(-350.0, 0.0),
            NodeData::Concept {
                text: "Cyberpunk Neo-Tokyo Chase".to_string(),
            },
        );
        let r1_id = self.add_node(
            Pos2::new(0.0, -150.0),
            NodeData::YouComResearch {
                query: "Neo-Tokyo architecture references".to_string(),
                result: None,
                is_loading: false,
            },
        );
        let p1_id = self.add_node(
            Pos2::new(350.0, 0.0),
            NodeData::Visual {
                prompt: "Futuristic police car in rain, neon lights".to_string(),
                texture: None,
                is_loading: false,
            },
        );
        let f1_id = self.add_node(
            Pos2::new(0.0, 250.0),
            NodeData::FoxitExport {
                status: "Ready".to_string(),
            },
        );

        self.state.edges.push(Edge { id: 1, from: c1_id, to: r1_id });
        self.state.edges.push(Edge { id: 2, from: r1_id, to: p1_id });
        self.state.edges.push(Edge { id: 3, from: p1_id, to: f1_id });
    }

    fn add_node(&mut self, pos: Pos2, data: NodeData) -> u64 {
        let id = self.state.next_id;
        self.state.nodes.insert(id, Node::new(id, pos, data));
        self.state.next_id += 1;
        id
    }

    fn trigger_research(&self, node_id: u64, query: String, ctx: egui::Context) {
        let tx = self.http_tx.clone();
        let body = serde_json::json!({"query": query});
        let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
        let mut request = ehttp::Request::post("/api/research", body_bytes);
        request.headers.insert("Content-Type", "application/json");
        
        ehttp::fetch(request, move |result| {
            match result {
                Ok(response) => {
                    let text = response.text().unwrap_or_default().to_string();
                    let _ = tx.send(AppMessage::TextResponse(node_id, text));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Error(node_id, e));
                }
            }
            ctx.request_repaint();
        });
    }

    fn trigger_visualize(&self, node_id: u64, prompt: String, ctx: egui::Context) {
        let tx = self.http_tx.clone();
        let url = format!(
            "https://image.pollinations.ai/prompt/{}?width=512&height=300&nologo=true",
            urlencoding::encode(&prompt)
        );
        
        ehttp::fetch(ehttp::Request::get(&url), move |result| {
            match result {
                Ok(response) => {
                    let _ = tx.send(AppMessage::ImageResponse(node_id, response.bytes));
                }
                Err(e) => {
                    let _ = tx.send(AppMessage::Error(node_id, e));
                }
            }
            ctx.request_repaint();
        });
    }

    fn apply_physics(&mut self) {
        let repulsion = 6000.0;
        let attraction = 0.02;
        let damping = 0.8;

        let node_ids: Vec<u64> = self.state.nodes.keys().copied().collect();
        let mut forces: HashMap<u64, Vec2> = node_ids.iter().map(|&id| (id, Vec2::ZERO)).collect();

        for &i in &node_ids {
            for &j in &node_ids {
                if i == j { continue; }
                let pos_i = self.state.nodes[&i].position;
                let pos_j = self.state.nodes[&j].position;
                let delta = pos_i - pos_j;
                let dist = delta.length().max(100.0);
                let force = delta.normalized() * (repulsion / (dist * dist));
                *forces.get_mut(&i).unwrap() += force;
            }
        }

        for edge in &self.state.edges {
            if let (Some(n1), Some(n2)) = (self.state.nodes.get(&edge.from), self.state.nodes.get(&edge.to)) {
                let delta = n2.position - n1.position;
                let dist = delta.length();
                let force = delta.normalized() * (dist - 400.0) * attraction;
                *forces.get_mut(&edge.from).unwrap() += force;
                *forces.get_mut(&edge.to).unwrap() -= force;
            }
        }

        for id in node_ids {
            if let Some(node) = self.state.nodes.get_mut(&id) {
                if self.state.dragging_node == Some(id) { continue; }
                node.velocity = (node.velocity + forces[&id]) * damping;
                node.position += node.velocity;
            }
        }
    }
}

impl eframe::App for StoryBoardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let start_time = Instant::now();

        while let Ok(msg) = self.http_rx.try_recv() {
            match msg {
                AppMessage::TextResponse(id, text) => {
                    if let Some(node) = self.state.nodes.get_mut(&id) {
                        if let NodeData::YouComResearch { result: r, is_loading, .. } = &mut node.data {
                            *r = Some(text);
                            *is_loading = false;
                        }
                    }
                }
                AppMessage::ImageResponse(id, bytes) => {
                    if let Some(node) = self.state.nodes.get_mut(&id) {
                        if let NodeData::Visual { texture, is_loading, .. } = &mut node.data {
                            *is_loading = false;
                            
                            if let Ok(image) = image::load_from_memory(&bytes) {
                                let size = [image.width() as usize, image.height() as usize];
                                let image_buffer = image.to_rgba8();
                                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                                    size,
                                    image_buffer.as_raw(),
                                );
                                
                                *texture = Some(ctx.load_texture(
                                    format!("node-image-{}", id),
                                    color_image,
                                    egui::TextureOptions::LINEAR
                                ));
                            }
                        }
                    }
                }
                AppMessage::Error(id, _err) => {
                    if let Some(node) = self.state.nodes.get_mut(&id) {
                        match &mut node.data {
                            NodeData::YouComResearch { is_loading, .. } => *is_loading = false,
                            NodeData::Visual { is_loading, .. } => *is_loading = false,
                            _ => {}
                        }
                    }
                }
            }
        }

        self.apply_physics();

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(15, 15, 15)))
            .show(ctx, |ui| {
                let canvas_rect = ui.max_rect();
                let (response, painter) = ui.allocate_painter(canvas_rect.size(), Sense::click_and_drag());

                let camera_offset = self.state.camera_offset;
                let camera_zoom = self.state.camera_zoom;

                let world_to_screen = |pos: Pos2| {
                    let center = canvas_rect.center();
                    let rel = (pos.to_vec2() - camera_offset) * camera_zoom;
                    center + rel
                };

                let screen_to_world = |pos: Pos2| {
                    let center = canvas_rect.center();
                    let rel = (pos - center) / camera_zoom;
                    Pos2::new(rel.x + camera_offset.x, rel.y + camera_offset.y)
                };

                if response.dragged() && self.state.dragging_node.is_none() {
                    self.state.camera_offset -= response.drag_delta() / camera_zoom;
                }

                let scroll_delta = ctx.input(|i| i.raw_scroll_delta.y);
                if scroll_delta != 0.0 {
                    if let Some(pointer_pos) = ctx.input(|i| i.pointer.hover_pos()) {
                        let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                        let world_pos_before = screen_to_world(pointer_pos);
                        let new_zoom = (self.state.camera_zoom * zoom_factor).clamp(0.05, 5.0);
                        self.state.camera_zoom = new_zoom;
                        let center = canvas_rect.center();
                        self.state.camera_offset = world_pos_before.to_vec2() - (pointer_pos - center) / self.state.camera_zoom;
                    }
                }

                if let Some(pointer_pos) = ctx.input(|i| i.pointer.interact_pos()) {
                    let world_pos = screen_to_world(pointer_pos);
                    if response.drag_started() {
                        for node in self.state.nodes.values_mut() {
                            if node.bounds().contains(world_pos) {
                                self.state.dragging_node = Some(node.id);
                                node.selected = true;
                            } else {
                                node.selected = false;
                            }
                        }
                    }
                }

                if response.drag_stopped() {
                    self.state.dragging_node = None;
                }

                if let Some(id) = self.state.dragging_node {
                    if let Some(node) = self.state.nodes.get_mut(&id) {
                        node.position += response.drag_delta() / camera_zoom;
                    }
                }

                for edge in &self.state.edges {
                    if let (Some(n1), Some(n2)) = (self.state.nodes.get(&edge.from), self.state.nodes.get(&edge.to)) {
                        let p1 = world_to_screen(n1.position + Vec2::new(n1.size.x, n1.size.y / 2.0));
                        let p2 = world_to_screen(n2.position + Vec2::new(0.0, n2.size.y / 2.0));
                        let cp_dist = (p2.x - p1.x).abs() * 0.5;
                        let c1 = p1 + Vec2::new(cp_dist, 0.0);
                        let c2 = p2 - Vec2::new(cp_dist, 0.0);

                        painter.add(egui::Shape::CubicBezier(egui::epaint::CubicBezierShape {
                            points: [p1, c1, c2, p2],
                            closed: false,
                            fill: Color32::TRANSPARENT,
                            stroke: Stroke::new(2.0, Color32::from_gray(80)).into(),
                        }));
                    }
                }

                let node_ids: Vec<u64> = self.state.nodes.keys().copied().collect();
                for id in node_ids {
                    let node = &self.state.nodes[&id];
                    let screen_pos = world_to_screen(node.position);
                    let screen_size = node.size * camera_zoom;
                    let node_rect = Rect::from_min_size(screen_pos, screen_size);

                    if !canvas_rect.intersects(node_rect) { continue; }

                    let frame = Frame::none()
                        .fill(Color32::from_gray(30))
                        .rounding(Rounding::same(8.0))
                        .stroke(Stroke::new(1.0, if node.selected { Color32::from_rgb(0, 200, 255) } else { Color32::from_gray(60) }))
                        .inner_margin(Margin::same(12.0));

                    let mut node_data = node.data.clone();
                    let mut node_data_changed = false;
                    let mut trigger_research = None;
                    let mut trigger_visualize = None;

                    ui.put(node_rect, |ui: &mut egui::Ui| {
                        frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                let (title, icon) = match &node_data {
                                    NodeData::Concept { .. } => ("Concept", "🧠"),
                                    NodeData::YouComResearch { .. } => ("You.com Research", "🌐"),
                                    NodeData::Visual { .. } => ("AI Visualizer", "🎨"),
                                    NodeData::FoxitExport { .. } => ("Foxit Export", "📄"),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(icon);
                                    ui.heading(title);
                                });

                                if camera_zoom < 0.4 { return; }
                                ui.separator();

                                match &mut node_data {
                                    NodeData::Concept { text } => {
                                        if ui.text_edit_multiline(text).changed() {
                                            node_data_changed = true;
                                        }
                                    }
                                    NodeData::YouComResearch { query, result, is_loading } => {
                                        ui.add(egui::TextEdit::singleline(query));
                                        if *is_loading {
                                            ui.spinner();
                                        } else if let Some(res) = result {
                                            ui.small(res);
                                        } else {
                                            if ui.button("Search Context").clicked() {
                                                *is_loading = true;
                                                node_data_changed = true;
                                                trigger_research = Some(query.clone());
                                            }
                                        }
                                    }
                                    NodeData::Visual { prompt, texture, is_loading } => {
                                        ui.add(egui::TextEdit::multiline(prompt).hint_text("Describe image..."));
                                        if ui.button("Generate Image").clicked() {
                                            *is_loading = true;
                                            node_data_changed = true;
                                            trigger_visualize = Some(prompt.clone());
                                        }
                                        if *is_loading {
                                            ui.spinner();
                                        } else if let Some(tex) = texture {
                                            ui.image(& *tex);
                                        }
                                    }
                                    NodeData::FoxitExport { status } => {
                                        ui.label(format!("Status: {}", status));
                                        if ui.button("Generate PDF Call Sheet").clicked() {}
                                    }
                                }
                            });
                        }).response
                    });

                    if node_data_changed {
                        if let Some(n) = self.state.nodes.get_mut(&id) {
                            n.data = node_data;
                        }
                    }
                    if let Some(q) = trigger_research {
                        self.trigger_research(id, q, ctx.clone());
                    }
                    if let Some(p) = trigger_visualize {
                        self.trigger_visualize(id, p, ctx.clone());
                    }
                }
            });

        let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
        self.frame_times.push(elapsed);
        if self.frame_times.len() > 60 { self.frame_times.remove(0); }
        ctx.request_repaint();
    }
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
            ..Default::default()
        };
        eframe::run_native(
            "StoryBoard AI",
            options,
            Box::new(|cc| Ok(Box::new(StoryBoardApp::new(cc)))),
        ).unwrap();
    }

    #[cfg(target_arch = "wasm32")]
    {
        let web_options = eframe::WebOptions::default();
        wasm_bindgen_futures::spawn_local(async move {
            let canvas = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            eframe::WebRunner::new()
                .start(canvas, web_options, Box::new(|cc| Ok(Box::new(StoryBoardApp::new(cc)))))
                .await
                .expect("failed to start eframe");
        });
    }
}
