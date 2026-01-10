use crate::core::Plot;
use bevy::prelude::*;

pub fn render_3d_plot_placeholder(
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

    // Placeholder for 3D plot
    commands.entity(plot_entity).with_children(|parent| {
        parent.spawn(Text2dBundle {
            text: Text::from_section(
                "3D Plot",
                TextStyle {
                    font_size: 16.0,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(0.0, 0.0, 1.0),
            ..default()
        });

        // Show trace count
        parent.spawn(Text2dBundle {
            text: Text::from_section(
                format!("{} trace(s)", plot.traces_3d.len()),
                TextStyle {
                    font_size: 12.0,
                    color: Color::GRAY,
                    ..default()
                },
            ),
            transform: Transform::from_xyz(0.0, -height * 0.4, 1.0),
            ..default()
        });
    });
}
