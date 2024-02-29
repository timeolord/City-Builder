use std::borrow::Cow;
use std::marker::PhantomData;
use std::mem::size_of;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use bevy::render::render_asset::RenderAssets;
use bevy::render::render_graph::{Node, RenderGraph, RenderLabel};
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CachedPipelineState, ComputePassDescriptor,
    ComputePipelineDescriptor, Maintain, MapMode, PipelineCache,
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::FallbackImage;
use bevy::render::{Render, RenderSet};
use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{NodeRunError, RenderGraphContext},
        render_resource::{AsBindGroup, BindGroup, CachedComputePipelineId},
        renderer::RenderContext,
        RenderApp,
    },
};
use itertools::Itertools;
use std::fmt::Debug;
use std::hash::Hash;
use uuid::Uuid;

#[derive(ExtractResource, Resource, AsBindGroup, Clone)]
pub struct TestResource {
    #[uniform(0, visibility(compute))]
    values: Vec4,
    #[storage(1, visibility(compute), buffer)]
    result: Buffer,
    mapped_bytes: Arc<RwLock<Vec<u8>>>,
}

impl ComputeShaderResource for TestResource {
    //The buffer that the compute shader will write to on the GPU side.
    //Must have usage as: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
    fn result_buffer(&self) -> &Buffer {
        &self.result
    }
    //This is where we can access the bytes that the compute shader wrote to the buffer.
    //We need Arc<RwLock<Vec<u8>>> because the render thread is not necessarily synchronized with the main thread,
    //and also we need to transfer the data from the render world.
    fn mapped_bytes(&self) -> &Arc<RwLock<Vec<u8>>> {
        &self.mapped_bytes
    }
}
pub trait ComputeShaderResource {
    fn result_buffer(&self) -> &Buffer;
    fn mapped_bytes(&self) -> &Arc<RwLock<Vec<u8>>>;
}

pub struct ComputeShaderPlugin;

impl Plugin for ComputeShaderPlugin {
    fn build(&self, app: &mut App) {
        let dispatch_size = [1, 1, 1];
        app.add_plugins(<ComputeShaderWorker<TestResource>>::plugin(
            "test.wgsl".to_string(),
            dispatch_size,
        ));
        app.add_systems(
            Update,
            test_resource_print.run_if(resource_exists::<TestResource>),
        );
    }
    fn finish(&self, app: &mut App) {
        let render_device = app.world.resource::<RenderDevice>();
        let test = TestResource {
            values: Vec4::new(1.0, 2.0, 3.0, 4.0),
            result: render_device.create_buffer(&BufferDescriptor {
                label: None,
                size: size_of::<f32>() as u64 * 4,
                usage: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
                mapped_at_creation: false,
            }),
            mapped_bytes: Arc::new(RwLock::new(vec![0u8; size_of::<f32>() as usize * 4])),
        };
        app.insert_resource(test);
    }
}

fn test_resource_print(test: Res<TestResource>) {
    let result: Vec<f32> = test
        .mapped_bytes
        .read()
        .unwrap()
        .chunks_exact(4)
        .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect_vec();
    println!("{:?}", result);
}

#[derive(Resource, ExtractResource, Clone)]
pub struct ComputeShaderWorker<InputType: AsBindGroup + Sync + Send + 'static + Resource> {
    shader: Handle<Shader>,
    pipeline_id: Option<CachedComputePipelineId>,
    bind_group: Option<BindGroup>,
    dispatch_size: [u32; 3],
    _phantom_data: PhantomData<InputType>,
}
impl<Worker: AsBindGroup + Sync + Send + 'static + Resource> Default
    for ComputeShaderWorker<Worker>
{
    fn default() -> Self {
        Self {
            shader: Handle::default(),
            pipeline_id: None,
            bind_group: None,
            dispatch_size: [1, 1, 1],
            _phantom_data: PhantomData,
        }
    }
}

impl<
        T: AsBindGroup
            + Sync
            + Send
            + 'static
            + Clone
            + Resource
            + ComputeShaderResource
            + ExtractResource,
    > ComputeWorker for ComputeShaderWorker<T>
{
    type Input = T;

    fn bind_group(&self) -> &Option<BindGroup> {
        &self.bind_group
    }
    fn bind_group_mut(&mut self) -> &mut Option<BindGroup> {
        &mut self.bind_group
    }
    fn shader(&self) -> &Handle<Shader> {
        &self.shader
    }
    fn shader_mut(&mut self) -> &mut Handle<Shader> {
        &mut self.shader
    }
    fn pipeline_id(&self) -> &Option<CachedComputePipelineId> {
        &self.pipeline_id
    }
    fn pipeline_id_mut(&mut self) -> &mut Option<CachedComputePipelineId> {
        &mut self.pipeline_id
    }
    fn dispatch_size(&self) -> [u32; 3] {
        self.dispatch_size
    }
    fn dispatch_size_mut(&mut self) -> &mut [u32; 3] {
        &mut self.dispatch_size
    }
}
impl<T: AsBindGroup + Sync + Send + 'static + Clone + Resource> ComputeShaderWorker<T> {
    fn plugin(shader_path: String, dispatch_size: [u32; 3]) -> ComputeWorkerPlugin<Self> {
        ComputeWorkerPlugin::new(shader_path, dispatch_size)
    }
}
pub enum ComputeShaderWorkerNodeState {
    Loading,
    Ready,
}

pub struct ComputeShaderWorkerNode<Worker: ComputeWorker> {
    _phantom_data: PhantomData<Worker>,
    state: ComputeShaderWorkerNodeState,
    staging_buffer: Option<Buffer>,
}
impl<Worker: ComputeWorker> Default for ComputeShaderWorkerNode<Worker> {
    fn default() -> Self {
        Self {
            _phantom_data: PhantomData,
            state: ComputeShaderWorkerNodeState::Loading,
            staging_buffer: None,
        }
    }
}
impl<Worker: ComputeWorker> Node for ComputeShaderWorkerNode<Worker> {
    fn update(&mut self, world: &mut World) {
        match self.state {
            ComputeShaderWorkerNodeState::Loading => {
                let worker = world.resource::<Worker>();
                let pipeline_cache = world.resource::<PipelineCache>();
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(worker.pipeline_id().unwrap())
                {
                    if world.get_resource::<Worker::Input>().is_some() {
                        let render_device = world.resource::<RenderDevice>();
                        self.staging_buffer =
                            Some(render_device.create_buffer(&BufferDescriptor {
                                label: None,
                                size: world.resource::<Worker::Input>().result_buffer().size(),
                                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                                mapped_at_creation: false,
                            }));
                        self.state = ComputeShaderWorkerNodeState::Ready;
                    }
                }
            }
            ComputeShaderWorkerNodeState::Ready => {
                let render_device = world.resource::<RenderDevice>();

                self.staging_buffer
                    .as_ref()
                    .unwrap()
                    .slice(..)
                    .map_async(MapMode::Read, |_| {});
                render_device.poll(Maintain::Wait);
                let bytes = self
                    .staging_buffer
                    .as_ref()
                    .unwrap()
                    .slice(..)
                    .get_mapped_range()
                    .iter()
                    .cloned()
                    .collect_vec();
                let input = world.resource_mut::<Worker::Input>();
                let mut current_bytes = input.mapped_bytes().write().unwrap();
                *current_bytes = bytes;
                /* *input.mapped_bytes_mut() = bytes; */

                /* let result: Vec<f32> = bytes
                    .chunks_exact(4)
                    .map(|chunk| f32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect_vec();
                println!("{:?}", result); */
                self.staging_buffer.as_ref().unwrap().unmap();
            }
        }
    }
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        match self.state {
            ComputeShaderWorkerNodeState::Loading => Ok(()),
            ComputeShaderWorkerNodeState::Ready => {
                let worker = world.resource::<Worker>();
                let input = world.resource::<Worker::Input>();
                let pipeline_cache = world.resource::<PipelineCache>();

                let command_encoder = render_context.command_encoder();
                {
                    let mut pass =
                        command_encoder.begin_compute_pass(&ComputePassDescriptor::default());
                    pass.set_bind_group(0, worker.bind_group().as_ref().unwrap(), &[]);

                    let pipeline = pipeline_cache
                        .get_compute_pipeline(worker.pipeline_id().unwrap())
                        .unwrap();
                    pass.set_pipeline(pipeline);
                    let dispatch_size = worker.dispatch_size();
                    pass.dispatch_workgroups(dispatch_size[0], dispatch_size[1], dispatch_size[2]);
                }
                command_encoder.copy_buffer_to_buffer(
                    input.result_buffer(),
                    0,
                    self.staging_buffer.as_ref().unwrap(),
                    0,
                    self.staging_buffer.as_ref().unwrap().size(),
                );
                Ok(())
            }
        }
    }
}

pub trait ComputeWorker: Sized + Sync + Send + 'static + Resource {
    type Input: AsBindGroup
        + Sync
        + Send
        + 'static
        + Clone
        + ExtractResource
        + Resource
        + ComputeShaderResource;

    fn bind_group(&self) -> &Option<BindGroup>;
    fn bind_group_mut(&mut self) -> &mut Option<BindGroup>;
    fn shader(&self) -> &Handle<Shader>;
    fn shader_mut(&mut self) -> &mut Handle<Shader>;
    fn pipeline_id(&self) -> &Option<CachedComputePipelineId>;
    fn pipeline_id_mut(&mut self) -> &mut Option<CachedComputePipelineId>;
    fn dispatch_size(&self) -> [u32; 3];
    fn dispatch_size_mut(&mut self) -> &mut [u32; 3];

    fn prepare_bind_group(
        mut worker: ResMut<Self>,
        input: ResMut<Self::Input>,
        gpu_images: Res<RenderAssets<Image>>,
        render_device: Res<RenderDevice>,
        fallback_image: Res<FallbackImage>,
    ) {
        let bind_group_layout = Self::Input::bind_group_layout(&render_device);
        let prepared_bind_group = input
            .as_bind_group(
                &bind_group_layout,
                &render_device,
                &gpu_images,
                &fallback_image,
            )
            .unwrap();
        *worker.bind_group_mut() = Some(prepared_bind_group.bind_group);
    }
}
#[derive(RenderLabel, Clone, Eq, PartialEq, Hash, Debug)]
struct ComputeShaderWorkerNodeLabel {
    id: u128,
}
pub struct ComputeWorkerPlugin<Worker> {
    shader_path: String,
    dispatch_size: [u32; 3],
    _phantom_data: PhantomData<Worker>,
}
impl<Worker> ComputeWorkerPlugin<Worker> {
    pub fn new(shader_path: String, dispatch_size: [u32; 3]) -> Self {
        Self {
            shader_path,
            dispatch_size,
            _phantom_data: PhantomData,
        }
    }
}

impl<Worker: ComputeWorker + Default> Plugin for ComputeWorkerPlugin<Worker> {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<Worker::Input>::default());

        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            Worker::prepare_bind_group
                .in_set(RenderSet::PrepareBindGroups)
                .run_if(resource_exists::<Worker::Input>),
        );

        let node = ComputeShaderWorkerNode::<Worker>::default();
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

        let id: u128 = Uuid::new_v4().as_u128();
        render_graph.add_node(ComputeShaderWorkerNodeLabel { id }, node);
    }
    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.init_resource::<Worker>();
        let world = &mut render_app.world;
        let shader_path = PathBuf::new().join("shaders").join(&self.shader_path);
        let shader = world.resource::<AssetServer>().load(shader_path);
        let render_device = world.resource::<RenderDevice>();
        let bind_group_layout = Worker::Input::bind_group_layout(&render_device);
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: vec![bind_group_layout],
            push_constant_ranges: vec![],
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("main"),
        });
        let mut worker = world.resource_mut::<Worker>();
        *worker.shader_mut() = shader;
        *worker.pipeline_id_mut() = Some(pipeline);
        *worker.dispatch_size_mut() = self.dispatch_size;
    }
}
