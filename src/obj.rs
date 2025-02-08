#[derive(Default, Debug)]
pub struct Model {
    pub vertices: Vec<f32>,
    pub indices: Vec<u16>,
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
                model.indices.push(parse_face(f0) - 1);
                model.indices.push(parse_face(f1) - 1);
                model.indices.push(parse_face(f2) - 1);
            }
            ["f", f0, f1, f2, f3] => {
                model.indices.push(parse_face(f0) - 1);
                model.indices.push(parse_face(f1) - 1);
                model.indices.push(parse_face(f2) - 1);

                model.indices.push(parse_face(f0) - 1);
                model.indices.push(parse_face(f2) - 1);
                model.indices.push(parse_face(f3) - 1);
            }
            _ => {}
        }
    }

    model
}

fn parse_face(index: &str) -> u16 {
    index.split("/").next().unwrap().parse().unwrap()
}
