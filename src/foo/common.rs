use bevy::prelude::*;

#[derive(Component)]
pub struct PlotRoot;

#[derive(Component)]
pub struct TraceRoot;

#[derive(Component)]
pub struct PlotTitle;

#[derive(Component)]
pub struct PlotBorder;

pub fn create_plot_container(
    commands: &mut Commands,
    position: Vec3,
    width: f32,
    height: f32,
    name: &str,
) -> Entity {
    commands
        .spawn((
            SpatialBundle::from_transform(Transform::from_translation(position)),
            PlotRoot,
            Name::new(name.to_string()),
        ))
        .id()
}

pub fn add_plot_border(
    plot_entity: Entity,
    commands: &mut Commands,
    width: f32,
    height: f32,
    color: Color,
    thickness: f32,
) {
    commands.entity(plot_entity).with_children(|parent| {
        parent.spawn((
            bevy_prototype_lyon::prelude::ShapeBundle {
                path: bevy_prototype_lyon::prelude::GeometryBuilder::build_as(
                    &bevy_prototype_lyon::prelude::shapes::Rectangle {
                        extents: Vec2::new(width, height),
                        origin: bevy_prototype_lyon::prelude::shapes::RectangleOrigin::Center,
                    },
                ),
                ..default()
            },
            bevy_prototype_lyon::prelude::Stroke::new(color, thickness),
            PlotBorder,
        ));
    });
}

pub fn add_plot_title(plot_entity: Entity, commands: &mut Commands, title: &str, height: f32) {
    commands.entity(plot_entity).with_children(|parent| {
        parent.spawn((
            Text2dBundle {
                text: Text::from_section(
                    title,
                    TextStyle {
                        font_size: 14.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ),
                transform: Transform::from_xyz(0.0, height / 2.0 - 15.0, 1.0),
                ..default()
            },
            PlotTitle,
        ));
    });
}

pub fn color_from_array(color: [f32; 4]) -> Color {
    Color::rgba(color[0], color[1], color[2], color[3])
}
