use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4};

#[derive(Default, Debug)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}

#[derive(Debug, Copy, Clone, Pod, Zeroable)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
}

pub fn load_model(model_content: &str) -> Model {
    let mut model = Model::default();

    let mut temp_vertices = Vec::new();
    let mut temp_normals = Vec::new();

    for line in model_content.lines() {
        let values = line.split(" ").collect::<Vec<_>>();
        match values.as_slice() {
            ["v", v0, v1, v2] => temp_vertices.push(Vec4::new(
                v0.parse().unwrap(),
                v1.parse().unwrap(),
                v2.parse().unwrap(),
                1.0,
            )),
            ["vn", n0, n1, n2] => temp_normals.push(Vec4::new(
                n0.parse().unwrap(),
                n1.parse().unwrap(),
                n2.parse().unwrap(),
                1.0,
            )),
            ["f", f0, f1, f2] => {
                let indices0 = parse_indices(f0);
                let indices1 = parse_indices(f1);
                let indices2 = parse_indices(f2);

                model.vertices.push(Vertex {
                    position: temp_vertices[indices0.0],
                    normal: temp_normals[indices0.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices1.0],
                    normal: temp_normals[indices1.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices2.0],
                    normal: temp_normals[indices2.1],
                });
            }
            ["f", f0, f1, f2, f3] => {
                let indices0 = parse_indices(f0);
                let indices1 = parse_indices(f1);
                let indices2 = parse_indices(f2);
                let indices3 = parse_indices(f3);

                model.vertices.push(Vertex {
                    position: temp_vertices[indices0.0],
                    normal: temp_normals[indices0.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices1.0],
                    normal: temp_normals[indices1.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices2.0],
                    normal: temp_normals[indices2.1],
                });

                model.vertices.push(Vertex {
                    position: temp_vertices[indices0.0],
                    normal: temp_normals[indices0.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices2.0],
                    normal: temp_normals[indices2.1],
                });
                model.vertices.push(Vertex {
                    position: temp_vertices[indices3.0],
                    normal: temp_normals[indices3.1],
                });
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
