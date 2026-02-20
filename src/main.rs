use eframe::egui;
use eframe::wasm_bindgen::JsCast;
use egui::{Color32, Pos2, Rect, Sense, Slider, Stroke, Vec2};
use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    pub id: u64,
    pub position: Pos2,
    pub radius: f32,
    pub selected: bool,
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
    pub next_node_id: u64,
    pub next_edge_id: u64,
    pub camera_offset: Vec2,
    pub camera_zoom: f32,
    pub dragging_node: Option<u64>,
    pub frame_times: Vec<f32>,
    pub node_count: usize,
    pub edge_count: usize,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            next_node_id: 1,
            next_edge_id: 1,
            camera_offset: Vec2::ZERO,
            camera_zoom: 1.0,
            dragging_node: None,
            frame_times: Vec::new(),
            node_count: 1000,
            edge_count: 1500,
        }
    }
}

impl CanvasState {
    pub fn screen_to_world(&self, screen_pos: Pos2, canvas_rect: Rect) -> Pos2 {
        let center = canvas_rect.center();
        let relative = screen_pos - center;
        let scaled = relative / self.camera_zoom;
        Pos2::new(
            scaled.x + self.camera_offset.x,
            scaled.y + self.camera_offset.y,
        )
    }

    pub fn world_to_screen(&self, world_pos: Pos2, canvas_rect: Rect) -> Pos2 {
        let center = canvas_rect.center();
        let relative = world_pos - self.camera_offset;
        let scaled = relative * self.camera_zoom;
        Pos2::new(scaled.x + center.x, scaled.y + center.y)
    }

    pub fn hit_test(&self, world_pos: Pos2) -> Option<u64> {
        for (id, node) in &self.nodes {
            let dist = (node.position - world_pos).length();
            if dist <= node.radius {
                return Some(*id);
            }
        }
        None
    }

    pub fn generate_benchmark_graph(&mut self, node_count: usize, edge_count: usize) {
        self.nodes.clear();
        self.edges.clear();
        self.next_node_id = 1;
        self.next_edge_id = 1;

        let spacing = 50.0;
        let cols = (node_count as f32).sqrt() as usize;

        for i in 0..node_count {
            let row = i / cols;
            let col = i % cols;
            let x = col as f32 * spacing + rand::random::<f32>() * 20.0;
            let y = row as f32 * spacing + rand::random::<f32>() * 20.0;

            let node = Node {
                id: self.next_node_id,
                position: Pos2::new(x, y),
                radius: 10.0 + rand::random::<f32>() * 5.0,
                selected: false,
            };
            self.nodes.insert(self.next_node_id, node);
            self.next_node_id += 1;
        }

        for _ in 0..edge_count {
            let from = (rand::random::<u64>() % node_count as u64) + 1;
            let to = (rand::random::<u64>() % node_count as u64) + 1;
            if from != to {
                let edge = Edge {
                    id: self.next_edge_id,
                    from,
                    to,
                };
                self.edges.push(edge);
                self.next_edge_id += 1;
            }
        }

        self.node_count = node_count;
        self.edge_count = edge_count;
    }

    pub fn apply_force_directed(&mut self, iterations: usize) {
        let repulsion_strength = 5000.0;
        let attraction_strength = 0.01;
        let damping = 0.9;

        let node_ids: Vec<u64> = self.nodes.keys().copied().collect();
        let mut velocities: HashMap<u64, Vec2> =
            node_ids.iter().map(|&id| (id, Vec2::ZERO)).collect();

        for _ in 0..iterations {
            let mut forces: HashMap<u64, Vec2> =
                node_ids.iter().map(|&id| (id, Vec2::ZERO)).collect();

            for &i in &node_ids {
                for &j in &node_ids {
                    if i == j {
                        continue;
                    }
                    let pos_i = self.nodes[&i].position;
                    let pos_j = self.nodes[&j].position;
                    let delta = pos_i - pos_j;
                    let dist = delta.length().max(1.0);
                    let force = delta.normalized() * (repulsion_strength / (dist * dist));
                    *forces.get_mut(&i).unwrap() += force;
                }
            }

            for edge in &self.edges {
                if let (Some(from_node), Some(to_node)) =
                    (self.nodes.get(&edge.from), self.nodes.get(&edge.to))
                {
                    let delta = to_node.position - from_node.position;
                    let dist = delta.length().max(1.0);
                    let force = delta.normalized() * (dist - 100.0) * attraction_strength;
                    *forces.get_mut(&edge.from).unwrap() += force;
                    *forces.get_mut(&edge.to).unwrap() -= force;
                }
            }

            for &id in &node_ids {
                if let Some(node) = self.nodes.get_mut(&id) {
                    let vel = velocities.get_mut(&id).unwrap();
                    *vel += forces[&id];
                    *vel *= damping;
                    node.position += *vel;
                }
            }
        }
    }
}

pub struct CanvasApp {
    state: CanvasState,
    show_metrics: bool,
}

impl Default for CanvasApp {
    fn default() -> Self {
        Self {
            state: CanvasState::default(),
            show_metrics: true,
        }
    }
}

impl eframe::App for CanvasApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame_start = Instant::now();

        egui::SidePanel::left("controls")
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Canvas Controls");
                ui.separator();

                ui.label("Benchmark Controls");
                if ui.button("Generate 1K Nodes").clicked() {
                    self.state.generate_benchmark_graph(1000, 1500);
                }
                if ui.button("Generate 10K Nodes").clicked() {
                    self.state.generate_benchmark_graph(10000, 15000);
                }
                if ui.button("Generate 50K Nodes").clicked() {
                    self.state.generate_benchmark_graph(50000, 75000);
                }

                ui.separator();
                ui.label("Physics Simulation");
                if ui.button("Run Force-Directed (100 iter)").clicked() {
                    self.state.apply_force_directed(100);
                }

                ui.separator();
                ui.label("Camera");
                ui.add(Slider::new(&mut self.state.camera_zoom, 0.1..=5.0).text("Zoom"));
                if ui.button("Reset Camera").clicked() {
                    self.state.camera_offset = Vec2::ZERO;
                    self.state.camera_zoom = 1.0;
                }

                ui.separator();
                ui.checkbox(&mut self.show_metrics, "Show Metrics");

                if self.show_metrics {
                    ui.separator();
                    ui.label("Metrics");
                    ui.label(format!("Nodes: {}", self.state.nodes.len()));
                    ui.label(format!("Edges: {}", self.state.edges.len()));
                    if !self.state.frame_times.is_empty() {
                        let avg = self.state.frame_times.iter().sum::<f32>()
                            / self.state.frame_times.len() as f32;
                        let min = self
                            .state
                            .frame_times
                            .iter()
                            .cloned()
                            .fold(f32::INFINITY, f32::min);
                        let max = self
                            .state
                            .frame_times
                            .iter()
                            .cloned()
                            .fold(f32::NEG_INFINITY, f32::max);
                        ui.label(format!("Frame Time: {:.2}ms (avg)", avg));
                        ui.label(format!("Min: {:.2}ms", min));
                        ui.label(format!("Max: {:.2}ms", max));
                        ui.label(format!("FPS: {:.0}", 1000.0 / avg));
                    }
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let canvas_rect = ui.max_rect();

            let (response, painter) =
                ui.allocate_painter(canvas_rect.size(), Sense::click_and_drag());

            let mut drag_delta = Vec2::ZERO;
            let mut mouse_world_pos = Pos2::ZERO;
            let mouse_pos = response.interact_pointer_pos();

            if let Some(mpos) = mouse_pos {
                mouse_world_pos = self.state.screen_to_world(mpos, canvas_rect);

                if response.drag_delta() != Vec2::ZERO {
                    drag_delta = response.drag_delta() / self.state.camera_zoom;
                }
            }

            if response.drag_started() {
                if let Some(node_id) = self.state.hit_test(mouse_world_pos) {
                    self.state.dragging_node = Some(node_id);
                    if let Some(node) = self.state.nodes.get_mut(&node_id) {
                        node.selected = true;
                    }
                }
            }

            if response.drag_stopped() {
                self.state.dragging_node = None;
            }

            if let Some(node_id) = self.state.dragging_node {
                if let Some(node) = self.state.nodes.get_mut(&node_id) {
                    node.position += drag_delta;
                }
            }

            if response.dragged() && self.state.dragging_node.is_none() {
                self.state.camera_offset -= drag_delta;
            }

            let scroll = ctx.input(|i| i.raw_scroll_delta);
            if scroll.y != 0.0 {
                let zoom_factor = 1.0 + scroll.y.abs() * 0.001;
                let new_zoom = if scroll.y < 0.0 {
                    self.state.camera_zoom * zoom_factor
                } else {
                    self.state.camera_zoom / zoom_factor
                };
                self.state.camera_zoom = new_zoom.clamp(0.1, 5.0);
            }

            for edge in &self.state.edges {
                if let (Some(from), Some(to)) = (
                    self.state.nodes.get(&edge.from),
                    self.state.nodes.get(&edge.to),
                ) {
                    let p1: Vec2 = self
                        .state
                        .world_to_screen(from.position, canvas_rect)
                        .to_vec2();
                    let p2: Vec2 = self
                        .state
                        .world_to_screen(to.position, canvas_rect)
                        .to_vec2();

                    let ctrl = (p1 + p2) / 2.0 + Vec2::new(0.0, 50.0);

                    let mut path = Vec::new();
                    let segments = 20;
                    for i in 0..=segments {
                        let t = i as f32 / segments as f32;
                        let t2 = t * t;
                        let t3 = t2 * t;
                        let mt = 1.0 - t;
                        let mt2 = mt * mt;
                        let mt3 = mt2 * mt;

                        let pos = mt3 * p1
                            + 3.0 * mt2 * t * ctrl
                            + 3.0 * mt * t2 * (p1 + p2) / 2.0
                            + t3 * p2;
                        path.push(egui::Pos2::new(pos.x, pos.y));
                    }

                    painter.add(egui::Shape::line(path, Stroke::new(1.0, Color32::GRAY)));
                }
            }

            for node in self.state.nodes.values() {
                let screen_pos = self.state.world_to_screen(node.position, canvas_rect);
                let screen_radius = node.radius * self.state.camera_zoom;

                let color = if node.selected {
                    Color32::from_rgb(100, 150, 255)
                } else {
                    Color32::from_rgb(200, 200, 200)
                };
                painter.add(egui::Shape::circle_filled(screen_pos, screen_radius, color));
                painter.add(egui::Shape::circle_stroke(
                    screen_pos,
                    screen_radius,
                    Stroke::new(1.0, Color32::BLACK),
                ));
            }

            let frame_elapsed = frame_start.elapsed().as_secs_f32() * 1000.0;
            self.state.frame_times.push(frame_elapsed);
            if self.state.frame_times.len() > 60 {
                self.state.frame_times.remove(0);
            }

            ctx.request_repaint();
        });
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
            "Canvas Rust egui",
            options,
            Box::new(|_cc| Ok(Box::new(CanvasApp::default()))),
        )
        .unwrap();
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
                .start(canvas, web_options, Box::new(|_cc| Ok(Box::new(CanvasApp::default()))))
                .await
                .expect("failed to start eframe");
        });
    }
}
