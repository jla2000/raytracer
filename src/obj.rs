#[derive(Default, Debug)]
pub struct Model {
    pub vertices: Vec<f32>,
    pub vertex_indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub normal_indices: Vec<u32>,
}

pub fn load_model(model_content: &str) -> Model {
    let mut model = Model::default();

    for line in model_content.lines() {
        let values = line.split(" ").collect::<Vec<_>>();
        match values.as_slice() {
            ["v", v0, v1, v2] => {
                model.vertices.push(v0.parse().unwrap());
                model.vertices.push(v1.parse().unwrap());
                model.vertices.push(v2.parse().unwrap());
            }
            ["f", f0, f1, f2] => {
                let face0 = parse_face(f0);
                let face1 = parse_face(f1);
                let face2 = parse_face(f2);

                model.vertex_indices.push(face0.0);
                model.vertex_indices.push(face1.0);
                model.vertex_indices.push(face2.0);

                model.normal_indices.push(face0.1);
                model.normal_indices.push(face1.1);
                model.normal_indices.push(face2.1);
            }
            ["f", f0, f1, f2, f3] => {
                let face0 = parse_face(f0);
                let face1 = parse_face(f1);
                let face2 = parse_face(f2);
                let face3 = parse_face(f3);

                model.vertex_indices.push(face0.0);
                model.vertex_indices.push(face1.0);
                model.vertex_indices.push(face2.0);

                model.vertex_indices.push(face0.0);
                model.vertex_indices.push(face2.0);
                model.vertex_indices.push(face3.0);

                model.normal_indices.push(face0.1);
                model.normal_indices.push(face1.1);
                model.normal_indices.push(face2.1);

                model.normal_indices.push(face0.1);
                model.normal_indices.push(face2.1);
                model.normal_indices.push(face3.1);
            }
            ["vn", n0, n1, n2] => {
                model.normals.push(n0.parse().unwrap());
                model.normals.push(n1.parse().unwrap());
                model.normals.push(n2.parse().unwrap());
            }
            _ => {}
        }
    }

    model
}

fn parse_face(index: &str) -> (u32, u32) {
    let values = index.split("/").collect::<Vec<_>>();
    (
        values[0].parse::<u32>().unwrap() - 1,
        values[2].parse::<u32>().unwrap() - 1,
    )
}
