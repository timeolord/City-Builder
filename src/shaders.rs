use std::borrow::Cow;
use std::marker::PhantomData;
use std::mem::swap;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use bevy::ecs::system::ReadOnlySystemParam;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_graph::{Node, RenderGraph, RenderLabel};
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CachedPipeline, CachedPipelineState, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, Maintain, MapMode, Pipeline, PipelineCache, PipelineDescriptor
};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::FallbackImage;
use bevy::render::{Extract, MainWorld, Render, RenderSet};
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

/* #[derive(ExtractResource, Resource, AsBindGroup, Clone)]
pub struct TestResource {
    #[uniform(0, visibility(compute))]
    values: Vec4,
    #[storage(1, visibility(compute), buffer)]
    result: Buffer,
    mapped_bytes: Arc<RwLock<Vec<u8>>>,
}

impl ComputeShaderResource for TestResource {
    fn result_buffer(&self) -> &Buffer {
        &self.result
    }
    fn mapped_bytes(&self) -> &Arc<RwLock<Vec<u8>>> {
        &self.mapped_bytes
    }
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
} */

pub trait ComputeShaderResource {
    //The buffer that the compute shader the shader will write to on the GPU side.
    //Must have usage as: BufferUsages::COPY_SRC | BufferUsages::STORAGE,
    fn result_buffer(&self) -> &Buffer;
    //This is where we can access the bytes that the compute shader wrote to the results buffer.
    //We need Arc<RwLock<Vec<u8>>> because the render thread is not necessarily synchronized with the main thread,
    //and also we need to transfer the data from the render world.
    fn mapped_bytes(&self) -> &Arc<RwLock<Vec<u8>>>;
    //The dispatch size of the compute shader.
    fn dispatch_size(&self) -> [u32; 3];
    //The condition that the compute shader will run under.
    fn run_condition(&self) -> &Arc<RwLock<ComputeShaderRunType>>;
    //Stops the compute shader from running, and cleans up the allocated memory.
    fn cleanup(&mut self) {
        let mut run_condition = self.run_condition().write().unwrap();
        *run_condition = ComputeShaderRunType::CleanUp;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComputeShaderRunType {
    EveryFrame,
    Once,
    Never,
    CleanUp,
}

#[derive(Resource, ExtractResource, Clone)]
pub struct ComputeShaderWorker<InputType: AsBindGroup + Sync + Send + 'static + Resource> {
    shader: Handle<Shader>,
    pipeline_id: Option<CachedComputePipelineId>,
    bind_group: Option<BindGroup>,
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
}
impl<T: AsBindGroup + Sync + Send + 'static + Clone + Resource> ComputeShaderWorker<T> {
    pub fn plugin<S: Into<String>>(shader_path: S) -> ComputeWorkerPlugin<Self> {
        ComputeWorkerPlugin::new(shader_path.into())
    }
}
pub enum ComputeShaderWorkerNodeState {
    Loading,
    Ready,
}

#[derive(Resource)]
pub struct AppPipelineCache {
    pub pipeline_cache: Vec<CachedPipeline>,
}
impl AppPipelineCache {
    #[inline]
    pub fn get_compute_pipeline(&self, id: CachedComputePipelineId) -> Option<&ComputePipeline> {
        if let CachedPipelineState::Ok(Pipeline::ComputePipeline(pipeline)) =
            &self.pipeline_cache[id.id()].state
        {
            Some(pipeline)
        } else {
            None
        }
    }
}

/* pub struct ComputeShaderWorkerNode<Worker: ComputeWorker> {
    _phantom_data: PhantomData<Worker>,
    state: ComputeShaderWorkerNodeState,
    staging_buffer: Option<Buffer>,
    bytes_buffer: Vec<u8>,
    ran_once_before: bool,
}
impl<Worker: ComputeWorker> Default for ComputeShaderWorkerNode<Worker> {
    fn default() -> Self {
        Self {
            _phantom_data: PhantomData,
            state: ComputeShaderWorkerNodeState::Loading,
            staging_buffer: None,
            bytes_buffer: vec![],
            ran_once_before: false,
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
                        self.bytes_buffer = vec![
                            0u8;
                            world.resource::<Worker::Input>().result_buffer().size()
                                as usize
                        ];
                        self.state = ComputeShaderWorkerNodeState::Ready;
                    }
                }
            }
            ComputeShaderWorkerNodeState::Ready => {
                let run_condition = {
                    let input = world.resource::<Worker::Input>();
                    *input.run_condition().read().unwrap()
                };
                let mut copy_results = || {
                    let render_device = world.resource::<RenderDevice>();
                    self.staging_buffer
                        .as_ref()
                        .unwrap()
                        .slice(..)
                        .map_async(MapMode::Read, |_| {});
                    render_device.poll(Maintain::Wait);
                    for (index, byte) in self
                        .staging_buffer
                        .as_ref()
                        .unwrap()
                        .slice(..)
                        .get_mapped_range()
                        .iter()
                        .cloned()
                        .enumerate()
                    {
                        self.bytes_buffer[index] = byte;
                    }
                    let input = world.resource_mut::<Worker::Input>();
                    let mut current_bytes = input.mapped_bytes().write().unwrap();
                    swap(&mut *current_bytes, &mut self.bytes_buffer);
                    self.staging_buffer.as_ref().unwrap().unmap();
                };
                match run_condition {
                    ComputeShaderRunType::EveryFrame => {
                        copy_results();
                    }
                    ComputeShaderRunType::Once => {
                        copy_results();
                        if !self.ran_once_before {
                            self.ran_once_before = true;
                        } else {
                            let input = world.resource_mut::<Worker::Input>();
                            *input.run_condition().write().unwrap() = ComputeShaderRunType::Never;
                        }
                    }
                    ComputeShaderRunType::Never | ComputeShaderRunType::CleanUp => {}
                }
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
                let input = world.resource::<Worker::Input>();
                let run_condition = *input.run_condition().read().unwrap();
                match run_condition {
                    ComputeShaderRunType::EveryFrame | ComputeShaderRunType::Once => {}
                    ComputeShaderRunType::Never | ComputeShaderRunType::CleanUp => return Ok(()),
                }

                let worker = world.resource::<Worker>();
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
                    let dispatch_size = input.dispatch_size();
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
} */

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

    fn prepare_bind_group(
        mut worker: ResMut<Self>,
        input: ResMut<Self::Input>,
        gpu_images: Res<RenderAssets<Image>>,
        render_device: Res<RenderDevice>,
        fallback_image: Res<FallbackImage>,
    ) {
        match *input.run_condition().read().unwrap() {
            ComputeShaderRunType::Never | ComputeShaderRunType::CleanUp => {
                *worker.bind_group_mut() = None;
            }
            ComputeShaderRunType::EveryFrame | ComputeShaderRunType::Once => {
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
    }
    fn cleanup(resource: Option<ResMut<Self::Input>>) {
        if let Some(resource) = resource {
            if *resource.run_condition().read().unwrap() == ComputeShaderRunType::CleanUp {
                resource.result_buffer().destroy();
                resource.mapped_bytes().write().unwrap().clear();
                *resource.run_condition().write().unwrap() = ComputeShaderRunType::Never;
            }
        }
    }
}
/* #[derive(RenderLabel, Clone, Eq, PartialEq, Hash, Debug)]
struct ComputeShaderWorkerNodeLabel {
    id: u128,
} */
pub struct ComputeWorkerPlugin<Worker> {
    shader_path: String,
    _phantom_data: PhantomData<Worker>,
}

impl<Worker> ComputeWorkerPlugin<Worker> {
    pub fn new(shader_path: String) -> Self {
        Self {
            shader_path,
            _phantom_data: PhantomData,
        }
    }
}

impl<Worker: ComputeWorker + Default> Plugin for ComputeWorkerPlugin<Worker> {
    fn build(&self, app: &mut App) {
        /* app.add_plugins(ExtractResourcePlugin::<Worker::Input>::default()); */

        /* let render_app = app.sub_app_mut(RenderApp);
        render_app.add_systems(
            Render,
            Worker::prepare_bind_group
                .in_set(RenderSet::PrepareBindGroups)
                .run_if(resource_exists::<Worker::Input>),
        );
        render_app.add_systems(Render, Worker::cleanup.in_set(RenderSet::Cleanup));

        let node = ComputeShaderWorkerNode::<Worker>::default();
        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

        let id: u128 = Uuid::new_v4().as_u128();
        render_graph.add_node(ComputeShaderWorkerNodeLabel { id }, node); */
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
    }
}
