use wgpu::{util::DeviceExt, CommandEncoderDescriptor, TextureViewDescriptor};
use winit::{dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop, window::WindowBuilder};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().expect("Não foi possível criar looping de eventos");
    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
        .with_resizable(false)
        .build(&event_loop)
        .expect("Não foi possível criar janela");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // Instanciar wgpu, necessário para criar surfaces e adapters
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    // Lugar que vai ser renderizado
    let surface = instance
        .create_surface(&window)
        .expect("Não foi possível criar surface parar renderizar");

    // Interface para a placa de vídeo, usado para pedir informações sobre a placa de video
    // Esta configuração é genérica, pode falhar caso não seja possível atender ela, nesse caso é
    // possível enumerar possiveis adaptadores com enumerate_adapters
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Não foi possível criar adaptador para renderizar");

    // Device é um handle para a GPU, informa as features e limitações do adapter carregado e cria
    // uma Queue de renderização também
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                required_features: adapter.features(),
                required_limits: adapter.limits(),
                label: None,
            },
            None,
        )
        .await
        .expect("Não foi possível criar device e queue para renderizar i");

    // Configurações que serão usadas quando esta Surface criar suas SurfaceTextures
    let surface_capabilities = surface.get_capabilities(&adapter);
    let surface_format = *surface_capabilities
        .formats
        .iter()
        .filter(|f| f.is_srgb())
        .next()
        .unwrap_or(&surface_capabilities.formats[0]);

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            // Qual vai ser o uso desta configuração, RENDER_ATTACHMENT significa que
            // esta surface pode ser usada como resultado de um render pass
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // Como esta SurfaceTexture vai ser guardado em memória, setamos para sRGB, ou o que
            // tiver
            format: surface_format,
            // Dimensões da textura
            width: WINDOW_WIDTH,
            height: WINDOW_HEIGHT,
            // Como sincronizar a surface com o display, Fifo basicamente é vsync
            present_mode: wgpu::PresentMode::Fifo,
            // Tutorial não soube explicar :S mantendo como tá lá
            alpha_mode: surface_capabilities.alpha_modes[0],
            // Usado quando envolver texturas
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    // Create pipeline
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor::default());
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            // Configura os buffer usado pelo vertex shader
            buffers: &[wgpu::VertexBufferLayout {
                // Cada buffer tem essa quantia de bytes de tamanho
                array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                // Honestamente, não entendi esse campo kkkkkkk
                step_mode: wgpu::VertexStepMode::Vertex,
                // O tipo de cada campo no VAO
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            // Configurações do fragment shader
            targets: &[Some(wgpu::ColorTargetState {
                // O formato das cores que o shader vai responder
                format: surface_format,
                // Substitui todos os pixels da textura
                blend: Some(wgpu::BlendState::REPLACE),
                // Relevante para texturas
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            // Requires Features::DEPTH_CLIP_CONTROL
            unclipped_depth: false,
            // Requires Features::CONSERVATIVE_RASTERIZATION
            conservative: false,
        },
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        depth_stencil: None,
        multiview: None,
    });

    // Cria buffer com os dados criados
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Triangulo buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let _ = event_loop.run(|event, target| match event {
        winit::event::Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => target.exit(),
        winit::event::Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let output = surface
                .get_current_texture()
                .expect("Não foi possível buscar textura para renderizar imagem");

            // Preciso buscar a view dessa textura para passar para a GPU
            let view = output
                .texture
                .create_view(&TextureViewDescriptor::default());
            // Buffer de comandos que são enviados para a GPU
            let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor::default());
            // Aqui vão todos os comandos que são passados para a GPU
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // Qual texture que vai ser desenhada, no caso a SurfaceTexture da janela
                    view: &view,
                    // Caso tenha multisampling, aqui vai a textura que vai receber o resultado,
                    // como setado para None, usa a mesma TextureView definida em view
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            render_pass.set_pipeline(&render_pipeline);
            // Diz pra renderizar este buffer
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            // Com essa quantidade de vertices
            render_pass.draw(0..VERTICES.len() as u32, 0..1);
            drop(render_pass);

            // Faz o submit dos render passs e apresenta
            queue.submit([encoder.finish()]);
            output.present();
        }
        _ => {}
    });
}
