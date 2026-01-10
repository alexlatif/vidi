use crate::core::{Plot, Point2D, Trace2D};
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

pub fn render_2d_plot(
    plot: &Plot,
    plot_entity: Entity,
    commands: &mut Commands,
    width: f32,
    height: f32,
) {
    // Add border
    super::common::add_plot_border(
        plot_entity,
        commands,
        width,
        height,
        Color::srgb(0.3, 0.3, 0.4),
        1.0,
    );

    // Add title
    super::common::add_plot_title(plot_entity, commands, &plot.config.title, height);

    // Calculate data bounds
    if let Some((min, max)) = plot.calculate_2d_bounds() {
        let [min_x, min_y] = min;
        let [max_x, max_y] = max;

        let data_width = max_x - min_x;
        let data_height = max_y - min_y;

        // Calculate scale (with 5% padding)
        let x_scale = width * 0.9 / data_width.max(f32::EPSILON);
        let y_scale = height * 0.9 / data_height.max(f32::EPSILON);
        let scale = x_scale.min(y_scale);

        // Render each trace
        for trace in &plot.traces_2d {
            if !trace.visible || trace.points.len() < 2 {
                continue;
            }

            render_2d_trace(
                trace,
                plot_entity,
                commands,
                width,
                height,
                min_x,
                min_y,
                scale,
            );
        }

        // Draw grid if enabled
        if plot.config.show_grid {
            draw_2d_grid(
                plot_entity,
                commands,
                width,
                height,
                min_x,
                max_x,
                min_y,
                max_y,
                scale,
            );
        }
    }
}

fn render_2d_trace(
    trace: &Trace2D,
    plot_entity: Entity,
    commands: &mut Commands,
    width: f32,
    height: f32,
    min_x: f32,
    min_y: f32,
    scale: f32,
) {
    let color = super::common::color_from_array(trace.color);

    // Convert points to local coordinates
    let points: Vec<Vec2> = trace
        .points
        .iter()
        .map(|p| {
            let local_x = (p.x - min_x) * scale - width * 0.45;
            let local_y = (p.y - min_y) * scale - height * 0.45;
            Vec2::new(local_x, local_y)
        })
        .collect();

    // Draw connected line (efficient with PathBuilder)
    if points.len() >= 2 {
        let mut path_builder = PathBuilder::new();
        path_builder.move_to(points[0]);

        for point in points.iter().skip(1) {
            path_builder.line_to(*point);
        }

        let path = path_builder.build();

        commands.entity(plot_entity).with_children(|parent| {
            parent.spawn((
                ShapeBundle { path, ..default() },
                Stroke::new(color, trace.width),
                super::common::TraceRoot,
            ));
        });
    }
}

fn draw_2d_grid(
    plot_entity: Entity,
    commands: &mut Commands,
    width: f32,
    height: f32,
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
    scale: f32,
) {
    let grid_color = Color::srgb(0.2, 0.2, 0.25);
    let grid_width = 0.5;

    // Calculate grid spacing
    let x_range = max_x - min_x;
    let y_range = max_y - min_y;

    let log_x = x_range.log10();
    let log_y = y_range.log10();

    let x_step = 10.0_f32.powf(log_x.floor()) / 10.0;
    let y_step = 10.0_f32.powf(log_y.floor()) / 10.0;

    // Draw vertical grid lines
    let mut x = (min_x / x_step).ceil() * x_step;
    while x <= max_x {
        let screen_x = (x - min_x) * scale - width * 0.45;

        commands.entity(plot_entity).with_children(|parent| {
            parent.spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shapes::Line(
                        Vec2::new(screen_x, -height * 0.45),
                        Vec2::new(screen_x, height * 0.45),
                    )),
                    ..default()
                },
                Stroke::new(grid_color, grid_width),
                super::common::TraceRoot,
            ));
        });

        x += x_step;
    }

    // Draw horizontal grid lines
    let mut y = (min_y / y_step).ceil() * y_step;
    while y <= max_y {
        let screen_y = (y - min_y) * scale - height * 0.45;

        commands.entity(plot_entity).with_children(|parent| {
            parent.spawn((
                ShapeBundle {
                    path: GeometryBuilder::build_as(&shapes::Line(
                        Vec2::new(-width * 0.45, screen_y),
                        Vec2::new(width * 0.45, screen_y),
                    )),
                    ..default()
                },
                Stroke::new(grid_color, grid_width),
                super::common::TraceRoot,
            ));
        });

        y += y_step;
    }
}
