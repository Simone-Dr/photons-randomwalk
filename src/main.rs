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
    let mut control = OrbitControl::new(*camera.target(), 0.1 * scene_radius, 2.0 * scene_radius);

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
   



    let ambient = AmbientLight::new(&context, 0.7, Color::WHITE);
    let directional0 = DirectionalLight::new(&context, 2.0, Color::WHITE, &vec3(-1.0, -1.0, -1.0));
    let directional1 = DirectionalLight::new(&context, 2.0, Color::WHITE, &vec3(1.0, 1.0, 1.0));

    let mut rw = RwSegment::new(&context, Color {r: 235, g: 201, b: 52, a: 200,});
    
    // main loop
    window.render_loop(move |mut frame_input| {
        camera.set_viewport(frame_input.viewport);
        control.handle_events(&mut camera, &mut frame_input.events);
        
        
        rw.next(5, 2);
        
        
        frame_input
            .screen()
            .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 0.0, 1.0))
            .render(
                &camera,
                model.into_iter().chain(&rw.gm).chain(&rw.gm_sphere),
                &[&ambient, &directional0, &directional1],
            );
        
        

        FrameOutput {
            swap_buffers: true,
            ..Default::default()
        }
    });
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
    fn new(context : &Context, col : Color) -> Self{
        let transformations:Vec<Mat4> = Vec::new();
        let inst = Instances {
            transformations,
            ..Default::default()
        };
        
        let physMat = PhysicalMaterial::new_transparent(
            &context,
            &CpuMaterial {
                albedo: col,
                ..Default::default()
            },
            );
        
        let tmp_gm= Gm::new(
        InstancedMesh::new(&(context.clone()), &inst , &CpuMesh::cylinder(16)),
        physMat.clone(),
        );
        
        RwSegment{
            f_pos: vec3(0.0, 0.0, 0.0),
            gm: tmp_gm,
            instances: inst,
            in_sun: true,
            steps: 0,
            gm_sphere: Gm::new(
        Mesh::new(&context, &CpuMesh::sphere(16)),
        physMat,),
        }
    }
    
    fn next(&mut self, steps_before_render: i64, steps_before_line: i64){
        if !self.in_sun {return};
        
        let mut rng = rand::thread_rng(); // random
        let val = 0.5;                          // how long each step is
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

