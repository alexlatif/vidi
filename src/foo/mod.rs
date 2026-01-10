mod common;
mod plot2d;
mod plot3d;

pub use common::*;
pub use plot2d::*;
pub use plot3d::*;

use crate::core::{Dashboard, Plot};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub struct DashboardPlugin;

impl Plugin for DashboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ShapePlugin)
            .add_systems(Startup, setup_camera)
            .add_systems(Update, render_dashboard);
    }
}

#[derive(Resource)]
pub struct DashboardState {
    pub dashboard: Dashboard,
}

impl DashboardState {
    pub fn new(dashboard: Dashboard) -> Self {
        Self { dashboard }
    }
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d::default());
    commands.spawn((SpatialBundle::default(), Name::new("DashboardRoot")));
}

fn render_dashboard(
    mut commands: Commands,
    dashboard_state: Res<DashboardState>,
    plot_query: Query<Entity, With<common::PlotRoot>>,
    windows: Query<&Window>,
) {
    // Clear previous plots
    for entity in plot_query.iter() {
        commands.entity(entity).despawn_recursive();
    }

    let dashboard = &dashboard_state.dashboard;
    let (rows, cols) = dashboard.grid;
    let window = windows.single();

    // Calculate layout
    let window_width = window.width();
    let window_height = window.height();

    let total_width =
        window_width - dashboard.padding * 2.0 - dashboard.spacing * (cols - 1) as f32;
    let total_height =
        window_height - dashboard.padding * 2.0 - dashboard.spacing * (rows - 1) as f32;

    let plot_width = total_width / cols as f32;
    let plot_height = total_height / rows as f32;

    // Set background color
    commands.insert_resource(ClearColor(bevy::prelude::Color::rgba(
        dashboard.background[0],
        dashboard.background[1],
        dashboard.background[2],
        dashboard.background[3],
    )));

    // Render each plot
    for plot in &dashboard.plots {
        let (row, col) = plot.config.position;

        // Calculate plot position
        let x =
            dashboard.padding + col as f32 * (plot_width + dashboard.spacing) + plot_width / 2.0;
        let y = dashboard.padding
            + (rows - row - 1) as f32 * (plot_height + dashboard.spacing)
            + plot_height / 2.0;

        // Create plot container
        let plot_entity = commands
            .spawn((
                SpatialBundle::from_transform(Transform::from_xyz(x, y, 0.0)),
                common::PlotRoot,
                Name::new(format!("Plot_{}_{}", row, col)),
            ))
            .id();

        // Render based on plot type
        match plot.plot_type {
            crate::core::PlotType::Plot2D => {
                plot2d::render_2d_plot(plot, plot_entity, &mut commands, plot_width, plot_height);
            }
            crate::core::PlotType::Plot3D => {
                plot3d::render_3d_plot_placeholder(
                    plot,
                    plot_entity,
                    &mut commands,
                    plot_width,
                    plot_height,
                );
            }
        }
    }
}
