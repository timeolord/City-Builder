use bevy::prelude::*;
use bevy_app_compute::prelude::AppComputeWorkerPlugin;

use self::blur::BlurComputeWorker;

pub mod blur;
pub mod direction;
pub mod math;

pub struct UtilPlugin;

impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AppComputeWorkerPlugin::<BlurComputeWorker>::default());
    }
}
