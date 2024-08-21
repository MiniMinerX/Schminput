use bevy::{input::mouse::MouseMotion, prelude::*};

use crate::{
    subaction_paths::{RequestedSubactionPaths, SubactionPathCreated, SubactionPathStr},
    BoolActionValue, ButtonInputBeheavior, F32ActionValue, InputAxis, InputAxisDirection,
    SchminputSet, Vec2ActionValue,
};

pub struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            sync_actions.in_set(SchminputSet::SyncInputActions),
        );
        app.add_systems(
            PreUpdate,
            handle_new_subaction_paths.in_set(SchminputSet::HandleNewSubactionPaths),
        );
    }
}

fn handle_new_subaction_paths(
    query: Query<&SubactionPathStr>,
    mut event: EventReader<SubactionPathCreated>,
    mut cmds: Commands,
) {
    for (entity, path) in event
        .read()
        .filter_map(|e| Some((e.0 .0, query.get(e.0 .0).ok()?)))
    {
        if let Some(sub_path) = path.0.strip_prefix("/mouse") {
            if sub_path.is_empty() || sub_path == "/*" {
                cmds.entity(entity).insert(MouseSubactionPath::All);
                continue;
            }
            if sub_path == "/motion" {
                cmds.entity(entity).insert(MouseSubactionPath::DeltaMotion);
                continue;
            }
            if sub_path == "/button" {
                cmds.entity(entity).insert(MouseSubactionPath::Button);
                continue;
            }
            // if sub_path == "/scroll" {
            //     cmds.entity(entity).insert(MouseSubactionPath::Scroll);
            //     continue;
            // }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn sync_actions(
    mut action_query: Query<(
        &MouseBindings,
        Option<&mut BoolActionValue>,
        Option<&mut F32ActionValue>,
        Option<&mut Vec2ActionValue>,
        &RequestedSubactionPaths,
    )>,
    path_query: Query<&MouseSubactionPath>,
    time: Res<Time>,
    input: Res<ButtonInput<MouseButton>>,
    mut delta_motion: EventReader<MouseMotion>,
) {
    for (binding, mut bool_value, mut f32_value, mut vec2_value, paths) in &mut action_query {
        for button in &binding.buttons {
            let paths = paths
                .iter()
                .filter_map(|e| Some((e, path_query.get(e.0).ok()?)))
                .filter(|(_, p)| {
                    **p == MouseSubactionPath::Button || **p == MouseSubactionPath::All
                })
                .map(|(e, _)| *e);

            let delta_mutiplier = match button.premultipy_delta_time {
                true => time.delta_seconds(),
                false => 1.0,
            };
            if let Some(boolean) = bool_value.as_mut() {
                *boolean.0 |= button.behavior.apply(&input, button.button);
                for path in paths.clone() {
                    *boolean.entry_with_path(path).or_default() |=
                        button.behavior.apply(&input, button.button);
                }
            }
            if let Some(float) = f32_value.as_mut() {
                let val = button.behavior.apply(&input, button.button) as u8 as f32;

                *float.0 += val * button.axis_dir.as_multipier() * delta_mutiplier;
                for path in paths.clone() {
                    *float.entry_with_path(path).or_default() +=
                        val * button.axis_dir.as_multipier() * delta_mutiplier;
                }
            }
            if let Some(vec) = vec2_value.as_mut() {
                let val = button.behavior.apply(&input, button.button) as u8 as f32;

                *button.axis.vec_axis_mut(vec) +=
                    val * button.axis_dir.as_multipier() * delta_mutiplier;
                for path in paths.clone() {
                    *button
                        .axis
                        .vec_axis_mut(vec.entry_with_path(path).or_default()) +=
                        val * button.axis_dir.as_multipier() * delta_mutiplier;
                }
            }
        }

        let Some(movement) = binding.movement else {
            continue;
        };
        let sub_paths = paths
            .iter()
            .filter_map(|e| Some((e, path_query.get(e.0).ok()?)))
            .filter(|(_, p)| {
                **p == MouseSubactionPath::DeltaMotion || **p == MouseSubactionPath::All
            })
            .map(|(e, _)| *e)
            .collect::<Vec<_>>();

        if movement.motion_type == MouseMotionType::DeltaMotion {
            let mut delta = Vec2::ZERO;
            for e in delta_motion.read() {
                let mut v = e.delta;
                v.y *= -1.0;
                delta += v * movement.multiplier;
            }
            if let Some(boolean) = bool_value.as_mut() {
                *boolean.0 |= delta != Vec2::ZERO;
                for path in sub_paths.iter() {
                    *boolean.entry_with_path(*path).or_default() |= delta != Vec2::ZERO;
                }
            }
            if let Some(float) = f32_value.as_mut() {
                *float.0 += delta.x;
                for path in sub_paths.iter() {
                    *float.entry_with_path(*path).or_default() += delta.x;
                }
            }
            if let Some(vec2) = vec2_value.as_mut() {
                *vec2.0 += delta;
                for path in sub_paths.iter() {
                    *vec2.entry_with_path(*path).or_default() += delta;
                }
            }
        }
    }
}

#[derive(Clone, Debug, Reflect, Component, Copy, PartialEq, Eq)]
pub enum MouseSubactionPath {
    DeltaMotion,
    Button,
    // Scroll,
    All,
}

#[derive(Clone, Default, Debug, Reflect, Component)]
pub struct MouseBindings {
    pub buttons: Vec<MouseButtonBinding>,
    pub movement: Option<MouseMotionBinding>,
}

impl MouseBindings {
    pub fn add_binding(mut self, binding: MouseButtonBinding) -> Self {
        self.buttons.push(binding);
        self
    }
    pub fn delta_motion(mut self) -> Self {
        let mut mmb = match self.movement {
            Some(v) => v,
            None => MouseMotionBinding {
                motion_type: MouseMotionType::DeltaMotion,
                multiplier: 1.0,
            },
        };
        mmb.motion_type = MouseMotionType::DeltaMotion;
        self.movement = Some(mmb);
        self
    }
    pub fn motion_multiplier(mut self, multiplier: f32) -> Self {
        let mut mmb = match self.movement {
            Some(v) => v,
            None => MouseMotionBinding {
                motion_type: MouseMotionType::DeltaMotion,
                multiplier: 1.0,
            },
        };
        mmb.multiplier = multiplier;
        self.movement = Some(mmb);
        self
    }
}

#[derive(Clone, Copy, Debug, Reflect)]
pub struct MouseButtonBinding {
    pub axis: InputAxis,
    pub axis_dir: InputAxisDirection,
    pub button: MouseButton,
    pub premultipy_delta_time: bool,
    pub behavior: ButtonInputBeheavior,
}

#[derive(Clone, Copy, Default, Debug, Reflect)]
pub struct MouseMotionBinding {
    pub motion_type: MouseMotionType,
    pub multiplier: f32,
}

#[derive(Clone, Copy, Default, Debug, Reflect, PartialEq, Eq)]
pub enum MouseMotionType {
    #[default]
    DeltaMotion,
}
