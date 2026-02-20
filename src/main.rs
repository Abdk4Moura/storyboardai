use eframe::egui;
use eframe::wasm_bindgen::JsCast;
use egui::{
    Color32, Frame, Margin, Pos2, Rect, Rounding, Sense, Stroke, Vec2,
};
use std::collections::HashMap;
use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum NodeData {
    Concept {
        text: String,
    },
    YouComResearch {
        query: String,
        result: Option<String>,
        is_loading: bool,
    },
    PerfectCorpImage {
        prompt: String,
        image_url: Option<String>,
        is_loading: bool,
    },
    FoxitExport {
        status: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: u64,
    pub position: Pos2,
    pub size: Vec2,
    pub data: NodeData,
    pub selected: bool,
    pub velocity: Vec2, // For physics
}

impl Node {
    pub fn new(id: u64, position: Pos2, data: NodeData) -> Self {
        Self {
            id,
            position,
            size: Vec2::new(200.0, 150.0),
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
    http_rx: mpsc::Receiver<(u64, String)>,
    http_tx: mpsc::Sender<(u64, String)>,
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
            Pos2::new(-300.0, 0.0),
            NodeData::Concept {
                text: "Cyberpunk Neo-Tokyo Chase".to_string(),
            },
        );
        let r1_id = self.add_node(
            Pos2::new(0.0, -100.0),
            NodeData::YouComResearch {
                query: "Neo-Tokyo architecture references".to_string(),
                result: None,
                is_loading: false,
            },
        );
        let p1_id = self.add_node(
            Pos2::new(300.0, 0.0),
            NodeData::PerfectCorpImage {
                prompt: "Futuristic police car in rain, neon lights".to_string(),
                image_url: None,
                is_loading: false,
            },
        );
        let f1_id = self.add_node(
            Pos2::new(0.0, 200.0),
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

    fn trigger_fetch(&self, node_id: u64, url: String, ctx: egui::Context) {
        let tx = self.http_tx.clone();
        let request = ehttp::Request::get(url);
        ehttp::fetch(request, move |result| {
            if let Ok(response) = result {
                let text = response.text().unwrap_or_default().to_string();
                let _ = tx.send((node_id, text));
                ctx.request_repaint();
            }
        });
    }

    fn apply_physics(&mut self) {
        let repulsion = 5000.0;
        let attraction = 0.02;
        let damping = 0.8;

        let node_ids: Vec<u64> = self.state.nodes.keys().copied().collect();
        let mut forces: HashMap<u64, Vec2> = node_ids.iter().map(|&id| (id, Vec2::ZERO)).collect();

        // Repulsion
        for &i in &node_ids {
            for &j in &node_ids {
                if i == j { continue; }
                let pos_i = self.state.nodes[&i].position;
                let pos_j = self.state.nodes[&j].position;
                let delta = pos_i - pos_j;
                let dist = delta.length().max(50.0);
                let force = delta.normalized() * (repulsion / (dist * dist));
                *forces.get_mut(&i).unwrap() += force;
            }
        }

        // Attraction
        for edge in &self.state.edges {
            if let (Some(n1), Some(n2)) = (self.state.nodes.get(&edge.from), self.state.nodes.get(&edge.to)) {
                let delta = n2.position - n1.position;
                let dist = delta.length();
                let force = delta.normalized() * (dist - 300.0) * attraction;
                *forces.get_mut(&edge.from).unwrap() += force;
                *forces.get_mut(&edge.to).unwrap() -= force;
            }
        }

        // Apply
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

        // Handle Async HTTP Responses
        while let Ok((id, result)) = self.http_rx.try_recv() {
            if let Some(node) = self.state.nodes.get_mut(&id) {
                match &mut node.data {
                    NodeData::YouComResearch { result: r, is_loading, .. } => {
                        *r = Some(result);
                        *is_loading = false;
                    }
                    NodeData::PerfectCorpImage { image_url, is_loading, .. } => {
                        *image_url = Some("https://picsum.photos/400/300".to_string());
                        *is_loading = false;
                    }
                    _ => {}
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

                // Input handling: Camera Panning
                if response.dragged() && self.state.dragging_node.is_none() {
                    self.state.camera_offset -= response.drag_delta() / camera_zoom;
                }

                // Input handling: Zoom-to-Cursor
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

                // Draw Edges (Bezier)
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

                // Draw Nodes
                let node_ids: Vec<u64> = self.state.nodes.keys().copied().collect();
                for id in node_ids {
                    let node = &self.state.nodes[&id];
                    let screen_pos = world_to_screen(node.position);
                    let screen_size = node.size * camera_zoom;
                    let node_rect = Rect::from_min_size(screen_pos, screen_size);

                    // Frustum Culling
                    if !canvas_rect.intersects(node_rect) {
                        continue;
                    }

                    let frame = Frame::none()
                        .fill(Color32::from_gray(30))
                        .rounding(Rounding::same(8.0))
                        .stroke(Stroke::new(1.0, if node.selected { Color32::from_rgb(0, 200, 255) } else { Color32::from_gray(60) }))
                        .inner_margin(Margin::same(12.0));

                    let mut node_data = node.data.clone();
                    let mut node_data_changed = false;
                    let mut trigger_fetch_id = None;
                    let mut trigger_fetch_url = None;

                    ui.put(node_rect, |ui: &mut egui::Ui| {
                        frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                // Header
                                let (title, icon) = match &node_data {
                                    NodeData::Concept { .. } => ("Concept", "🧠"),
                                    NodeData::YouComResearch { .. } => ("You.com Research", "🌐"),
                                    NodeData::PerfectCorpImage { .. } => ("Perfect Corp Visual", "🎨"),
                                    NodeData::FoxitExport { .. } => ("Foxit Export", "📄"),
                                };
                                ui.horizontal(|ui| {
                                    ui.label(icon);
                                    ui.heading(title);
                                });

                                // Level of Detail (LOD) check
                                if camera_zoom < 0.4 {
                                    return;
                                }

                                ui.separator();

                                // Content
                                match &mut node_data {
                                    NodeData::Concept { text } => {
                                        if ui.text_edit_multiline(text).changed() {
                                            node_data_changed = true;
                                        }
                                    }
                                    NodeData::YouComResearch { query, result, is_loading } => {
                                        ui.label(format!("Query: {}", query));
                                        if *is_loading {
                                            ui.spinner();
                                        } else if let Some(res) = result {
                                            ui.small(res);
                                        } else {
                                            if ui.button("Search Context").clicked() {
                                                *is_loading = true;
                                                node_data_changed = true;
                                                trigger_fetch_id = Some(id);
                                                trigger_fetch_url = Some(format!("https://api.ydc-index.io/search?query={}", query));
                                            }
                                        }
                                    }
                                    NodeData::PerfectCorpImage { prompt, image_url, is_loading } => {
                                        ui.label(format!("Prompt: {}", prompt));
                                        if *is_loading {
                                            ui.spinner();
                                        } else if let Some(url) = image_url {
                                            ui.label("Image Generated!");
                                            ui.small(url);
                                            ui.painter().rect_stroke(ui.available_rect_before_wrap(), 4.0, Stroke::new(1.0, Color32::GRAY));
                                        } else {
                                            if ui.button("Generate Storyboard Image").clicked() {
                                                *is_loading = true;
                                                node_data_changed = true;
                                                trigger_fetch_id = Some(id);
                                                trigger_fetch_url = Some("https://api.perfectcorp.com/v1/generate".to_string());
                                            }
                                        }
                                    }
                                    NodeData::FoxitExport { status } => {
                                        ui.label(format!("Status: {}", status));
                                        ui.add_space(10.0);
                                        if ui.button("Generate PDF Call Sheet").clicked() {
                                            // Trigger Foxit logic
                                        }
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
                    if let (Some(f_id), Some(f_url)) = (trigger_fetch_id, trigger_fetch_url) {
                        self.trigger_fetch(f_id, f_url, ctx.clone());
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
