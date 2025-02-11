use bytemuck::{Pod, Zeroable};
use glam::Vec3;

#[derive(Default, Debug)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub _pad0: f32,
    pub normal: Vec3,
    pub material: u32,
}

impl Vertex {
    fn new(position: Vec3, normal: Vec3, material: u32) -> Self {
        Self {
            position,
            normal,
            material,
            _pad0: 0.0,
        }
    }
}

pub fn load_model(model_content: &str) -> Model {
    let mut model = Model::default();

    let mut temp_vertices = Vec::new();
    let mut temp_normals = Vec::new();
    let mut temp_material_num = 0;

    for line in model_content.lines() {
        let values = line.split(" ").collect::<Vec<_>>();
        match values.as_slice() {
            ["v", v0, v1, v2] => temp_vertices.push(Vec3::new(
                v0.parse().unwrap(),
                v1.parse().unwrap(),
                v2.parse().unwrap(),
            )),
            ["vn", n0, n1, n2] => temp_normals.push(Vec3::new(
                n0.parse().unwrap(),
                n1.parse().unwrap(),
                n2.parse().unwrap(),
            )),
            ["usemtl", material_name] => {
                temp_material_num = match *material_name {
                    "BMW_E30_M3_WINDOWS" => 1,
                    "BMW_E30_M3_CHROME" => 1,
                    "BMW_E30_M3_LENS" => 1,
                    "BMW_E30_M3_SIDE_MIRROR" => 1,
                    "BMW_E30_M3_RIM" => 1,
                    "BMW_E30_M3_EMBLEMS" => 1,
                    "BMW_E30_M3_HEADLIGHT_REFLECTOR" => 1,
                    "BMW_E30_M3_TAILLIGHT_REFLECTOR" => 1,
                    "BMW_E30_M3_PLASTIC" => 1,
                    "Brake_Disc" => 1,
                    "Brembo_Calipers" => 1,
                    "Logo_Plane" => 1,
                    _ => 0,
                };
            }
            ["f", f0, f1, f2] => {
                let indices0 = parse_indices(f0);
                let indices1 = parse_indices(f1);
                let indices2 = parse_indices(f2);

                model.vertices.push(Vertex::new(
                    temp_vertices[indices0.0],
                    temp_normals[indices0.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices1.0],
                    temp_normals[indices1.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices2.0],
                    temp_normals[indices2.1],
                    temp_material_num,
                ));
            }
            ["f", f0, f1, f2, f3] => {
                let indices0 = parse_indices(f0);
                let indices1 = parse_indices(f1);
                let indices2 = parse_indices(f2);
                let indices3 = parse_indices(f3);

                model.vertices.push(Vertex::new(
                    temp_vertices[indices0.0],
                    temp_normals[indices0.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices1.0],
                    temp_normals[indices1.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices2.0],
                    temp_normals[indices2.1],
                    temp_material_num,
                ));

                model.vertices.push(Vertex::new(
                    temp_vertices[indices0.0],
                    temp_normals[indices0.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices2.0],
                    temp_normals[indices2.1],
                    temp_material_num,
                ));
                model.vertices.push(Vertex::new(
                    temp_vertices[indices3.0],
                    temp_normals[indices3.1],
                    temp_material_num,
                ));
            }
            _ => {}
        }
    }

    model
}

fn parse_indices(index: &str) -> (usize, usize) {
    let values = index.split("/").collect::<Vec<_>>();
    (
        values[0].parse::<usize>().unwrap() - 1,
        values[2].parse::<usize>().unwrap() - 1,
    )
}
