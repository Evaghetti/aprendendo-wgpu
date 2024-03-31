use wgpu::{util::DeviceExt, CommandEncoderDescriptor, TextureViewDescriptor};
use winit::{dpi::LogicalSize, event::WindowEvent, event_loop::EventLoop, window::WindowBuilder};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    texture_coord: [f32; 2],
}

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
        texture_coord: [0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
        texture_coord: [0.0, 1.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
        texture_coord: [1.0, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 0.0, 1.0],
        texture_coord: [1.0, 0.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 2, 3, 0];

#[tokio::main]
async fn main() {
    let event_loop = EventLoop::new().expect("Não foi possível criar looping de eventos");
    let window = WindowBuilder::new()
        .with_title("WGPU da desgraçaaaa")
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

    // Lê a imagem de disco
    let image = image::io::Reader::open("res/container.jpg")
        .expect("Não foi possível abrir imagem")
        .decode()
        .expect("Erro ao processar imagem");
    let texture_size = wgpu::Extent3d {
        width: image.width(),
        height: image.height(),
        depth_or_array_layers: 1,
    };
    // Cria uma textura
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Container"),
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        // Textura pode ser usada num binding group e pode receber dados no write_texture
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        sample_count: 1,
        size: texture_size,
        // Dimensões dessa imagem (2D)
        dimension: wgpu::TextureDimension::D2,
        view_formats: &[],
        // Quantidade de mipmap? O tutorial n explicou, deixando como tá lá
        mip_level_count: 1,
    });

    // Passa os dados da textura pra textura criada
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image.to_rgba8(),
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * image.width()),
            rows_per_image: Some(image.height()),
        },
        texture_size,
    );

    // Cria uma texture view (tipo um ponteiro pra textura, se eu entendi direito)
    // E um sampler pra ser usado no shader, o sampler informa como o shader deve renderizar a
    // imagem na textura
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("Texture sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    // Cria o layout do bind group
    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture bind group layout"),
            entries: &[
                // Duas entradas, uma pra textura, e outra pro sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // Só pode ser usado no fragment shader
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    // Só pode ser usado no fragment shader
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    // necessário bater com o sample_type da textura, não sei o que o filtering
                    // quer dizer
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                },
            ],
        });
    // Cria o bind groupd com o layout criado e sampler e view criados
    let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Bind Group"),
        layout: &texture_bind_group_layout,
        entries: &[
            // Aqui são os dados que equivalem o que foi configurado no layout
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&texture_sampler),
            },
        ],
    });

    // Create pipeline
    let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&texture_bind_group_layout],
        ..Default::default()
    });
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
                attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2],
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

    // Cria um vertex buffer com os dados de cada vertice
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Triangulo buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // Cria o index buffer para reaproveitar vertices em formas mais complexas
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Triangulo buffer indices"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
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
            // Seta o bind group para o bind group criado
            render_pass.set_bind_group(0, &texture_bind_group, &[]);
            // Diz pra renderizar este buffer
            render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            // Além de setar o vertex buffer, seta o index buffer
            render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // Renderiza baseado nos indices informados antes
            render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
            drop(render_pass);

            // Faz o submit dos render passs e apresenta
            queue.submit([encoder.finish()]);
            output.present();
        }
        _ => {}
    });
}
