use rand::Rng;
use std::{f32::consts::PI};
use three_d::*;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    run().await;
}

pub async fn run() {
    // Create a window (a canvas on web)
    let window = Window::new(WindowSettings {
        title: "test".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();

    // Get the graphics context from the window
    let context = window.gl();

    // Create a camera
    let target = vec3(0.0, 0.0, 0.0);
    let scene_radius:f32 = 150.0;
    let mut camera = Camera::new_perspective(
        window.viewport(),
        target + scene_radius * vec3(0.6, 0.3, 1.0).normalize(),
        target,
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        10000.0,
    );
    let mut control = OrbitControl::new(*camera.target(), 0.1 * scene_radius, 1.5 * scene_radius);

    // Load Sphere object
    let mut loaded = three_d_asset::io::load_async(&["assets/sphere.obj"])
        .await
        .unwrap();

    // Create Cpu mesh
    let mut cpu_mesh: CpuMesh = loaded.deserialize("sphere.obj").unwrap();
    cpu_mesh
        .transform(&(Mat4::from_translation(vec3(0.0, 0.0, 0.0))*Mat4::from_scale(100.0)))
        .unwrap();
    let mut model_material = PhysicalMaterial::new_opaque(
        &context,
        &CpuMaterial {
            albedo: Color::new_opaque(50, 50, 50),
            roughness: 0.7,
            metallic: 0.8,
            ..Default::default()
        },
    );
     
    model_material.render_states.cull = Cull::Back;
    let model = Gm::new(Mesh::new(&context, &cpu_mesh), model_material);
    let mut wireframe_material = PhysicalMaterial::new_opaque(
        &context,
        &CpuMaterial {
            albedo: Color::new_opaque(50, 50, 50),
            roughness: 0.7,
            metallic: 0.8,
            ..Default::default()
        },
    );
    wireframe_material.render_states.cull = Cull::Back;
    let mut cylinder = CpuMesh::cylinder(10);
    cylinder
        .transform(&Mat4::from_nonuniform_scale(1.0, 0.0001, 0.0001))
        .unwrap();
    let edges = Gm::new(
        InstancedMesh::new(&context, &edge_transformations(&cpu_mesh), &cylinder),
        wireframe_material.clone(),
    );

    let mut sphere = CpuMesh::sphere(8);
    sphere.transform(&Mat4::from_scale(0.01)).unwrap();
    let vertices = Gm::new(
        InstancedMesh::new(&context, &vertex_transformations(&cpu_mesh), &sphere),
        wireframe_material,
    );

    let ambient = AmbientLight::new(&context, 0.7, Color::WHITE);
    let directional0 = DirectionalLight::new(&context, 2.0, Color::WHITE, &vec3(-1.0, -1.0, -1.0));
    let directional1 = DirectionalLight::new(&context, 2.0, Color::WHITE, &vec3(1.0, 1.0, 1.0));

    let mut rw = RwSegment::new(&context);
    
    // main loop
    window.render_loop(move |mut frame_input| {
        let mut redraw = frame_input.first_frame;
        redraw |= camera.set_viewport(frame_input.viewport);
        redraw |= control.handle_events(&mut camera, &mut frame_input.events);
        redraw = true;
        
        rw.next(100, 1000);
        
        
        if redraw { // if first frame, or changed viewport or event
            frame_input
                .screen()
                .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
                .render(
                    &camera,
                    model.into_iter().chain(&vertices).chain(&edges).chain(&rw.gm).chain(&rw.gm_sphere),
                    &[&ambient, &directional0, &directional1],
                );
        }
        

        FrameOutput {
            swap_buffers: true,
            ..Default::default()
        }
    });
}

fn vertex_transformations(cpu_mesh: &CpuMesh) -> Instances {
    Instances {
        transformations: cpu_mesh
            .positions
            .to_f32()
            .into_iter()
            .map(|p| Mat4::from_translation(p))  //Transformationmatrix chich translates w/ p
            .collect(),
        ..Default::default()
    }
}

fn edge_transformations(cpu_mesh: &CpuMesh) -> Instances {
    let indices = cpu_mesh.indices.to_u32().unwrap();   // get indices of mesh (u32), three cont. indices define a triangle 
    let positions = cpu_mesh.positions.to_f32();    // get positions. vector of (3*f32)
    let mut transformations = Vec::new();   // transformation matrix. vector of (4 f32 x 4 f32)
    let mut keys = Vec::new();      // keys (sorted pairs of indices)
    for f in 0..indices.len() / 3 {        //iterate through one third of indices
        let mut fun = |i1, i2| {
            let key = if i1 < i2 { (i1, i2) } else { (i2, i1) }; // to check, wheater edge has been transformed before
            if !keys.contains(&key) { 
                keys.push(key);
                let p1: Vec3 = positions[i1];
                let p2: Vec3 = positions[i2];
                transformations.push(
                    Mat4::from_translation(p1)
                        * Into::<Mat4>::into(Quat::from_arc( //finds quaternion representing roation from 1,0,0 to p2-p1
                            vec3(1.0, 0.0, 0.0),
                            (p2 - p1).normalize(),
                            None,
                        ))
                        * Mat4::from_nonuniform_scale((p1 - p2).magnitude(), 1.0, 1.0), // Translation * Rotation * Scaling
                );
            }
        };
        let i1 = indices[3 * f] as usize;       //together i1, i2, i3 define a triangle
        let i2 = indices[3 * f + 1] as usize;
        let i3 = indices[3 * f + 2] as usize;
        fun(i1, i2);
        fun(i2, i3);
        fun(i3, i1);
    }
    Instances {
        transformations,
        ..Default::default()
    }
}


fn get_transformation(start_pos: Vec3, end_pos: Vec3) -> Matrix4<f32> {
    let transformation = Mat4::from_translation(start_pos)
                        * Into::<Mat4>::into(Quat::from_arc( //finds quaternion representing roation from 1,0,0 to p2-p1
                            vec3(1.0, 0.0, 0.0),
                            (end_pos - start_pos).normalize(),
                            None,
                        ))
                        * Mat4::from_nonuniform_scale((start_pos - end_pos).magnitude(), 1.0, 1.0) * Mat4::from_nonuniform_scale(1.0, 0.03, 0.03); // Translation * Rotation * Scaling
    transformation
}

struct RwSegment{
    f_pos: Vec3, 
    instances: Instances,
    gm: Gm<InstancedMesh, PhysicalMaterial>,
    in_sun: bool,
    steps: i64,
    gm_sphere: Gm<Mesh, PhysicalMaterial>,
}

impl RwSegment {
    fn new(context : &Context) -> Self{
        let transformations:Vec<Mat4> = Vec::new();
        let inst = Instances {
            transformations,
            ..Default::default()
        };
        
        let tmp_gm= Gm::new(
        InstancedMesh::new(&(context.clone()), &inst , &CpuMesh::cylinder(16)),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Color {
                    r: 235,  
                    g: 201,
                    b: 52,
                    a: 200,
                },
                ..Default::default()
            },
            ),
        );
        
        RwSegment{
            f_pos: vec3(0.0, 0.0, 0.0),
            gm: tmp_gm,
            instances: inst,
            in_sun: true,
            steps: 0,
            gm_sphere: Gm::new(
        Mesh::new(&context, &CpuMesh::sphere(16)),
        PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: Color {
                    r: 0,
                    g: 0,
                    b: 255,
                    a: 200,
                },
                ..Default::default()
            },
            ),
            ),
        }
    }
    
    fn next(&mut self, steps_before_render: i64, steps_before_line: i64){
        if !self.in_sun {return};
        
        let mut rng = rand::thread_rng(); // random
        let val = 1e-2;                          // how long each step is
        for _ in 0..steps_before_render {      // as often as how many lines are drawn at once 
            let start_pos = self.f_pos;
            let mut new_pos= vec3(0.0, 0.0, 0.0);
            for _ in 0..steps_before_line{     // as often as how many lines are drawn as one line
                if self.in_sun{
                    let theta = rng.gen_range(0.0..2.0*PI); 
                    let phi = rng.gen_range(0.0..PI);
                    new_pos = vec3(val*theta.sin()*phi.cos(), val*theta.sin()*phi.sin(), val*theta.cos());       
                    new_pos = self.f_pos + new_pos;
                    self.f_pos = new_pos;                        
                    self.steps += 1;
                    if self.f_pos.magnitude() > 100.0{
                        self.in_sun = false;
                        print!("{:?}", self.steps);
                    }
                } else {break;}
            }
            if self.in_sun {       
                let new_trans = get_transformation(start_pos, new_pos);   
                self.gm_sphere.set_transformation(Mat4::from_translation(self.f_pos)*Mat4::from_scale(2.0));
                self.instances.transformations.push(new_trans);
                self.gm.set_instances(&self.instances);
            } else {break;}
                
                
            }
        }
    }

