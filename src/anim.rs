use bevy::prelude::*;
use bevy_aseprite_reader::{raw::AsepriteAnimationDirection, AsepriteInfo};

use crate::Aseprite;

#[derive(Component, Debug)]
pub struct AsepriteAnimation {
    pub tag: Option<String>,
    pub current_frame: usize,
    pub current_timer: f32,
    pub playing: bool,
    pub forward: bool,
}

impl Default for AsepriteAnimation {
    fn default() -> Self {
        Self {
            tag: default(),
            current_frame: default(),
            current_timer: default(),
            playing: true,
            forward: default(),
        }
    }
}

impl AsepriteAnimation {
    pub fn new<T, U>(info: &AsepriteInfo, tag: T) -> Self
    where
        T: Into<Option<U>>,
        U: Into<AsepriteTag>,
    {
        let tag = tag.into().map(|t| t.into().0);

        let (current_frame, current_timer, forward) = tag
            .as_ref()
            .and_then(|tag| {
                info.tags.get(tag).map(|tag| {
                    let (current_frame, forward) = match tag.animation_direction {
                        AsepriteAnimationDirection::Forward
                        | AsepriteAnimationDirection::PingPong => (tag.frames.start as usize, true),
                        AsepriteAnimationDirection::Reverse => (tag.frames.end as usize - 1, false),
                    };
                    let current_timer = info.frame_infos[current_frame].delay_ms as f32 / 1000.0;
                    (current_frame, current_timer, forward)
                })
            })
            .unwrap_or_else(|| {
                let current_frame = 0;
                let forward = true;
                let current_timer = info.frame_infos[current_frame].delay_ms as f32 / 1000.0;
                (current_frame, current_timer, forward)
            });

        Self {
            tag,
            current_frame,
            current_timer,
            playing: true,
            forward,
        }
    }

    /// Returns whether the frame was changed
    fn update(&mut self, info: &AsepriteInfo, dt: f32) -> bool {
        if self.is_paused() {
            return false;
        }

        self.current_timer -= dt;
        let mut changed = false;

        while self.current_timer <= 0.0 {
            self.next_frame(info);
            self.current_timer += self.current_frame_duration(info);
            changed = true;
        }

        changed
    }

    fn next_frame(&mut self, info: &AsepriteInfo) {
        match &self.tag {
            Some(tag) => {
                let tag = match info.tags.get(tag) {
                    Some(tag) => tag,
                    None => {
                        error!("Tag {} wasn't found.", tag);
                        return;
                    }
                };

                match tag.animation_direction {
                    AsepriteAnimationDirection::Forward => {
                        let next_frame = self.current_frame + 1;
                        if tag.frames.contains(&(next_frame as u16)) {
                            self.current_frame = next_frame;
                        } else {
                            self.current_frame = tag.frames.start as usize;
                        }
                    }
                    AsepriteAnimationDirection::Reverse => {
                        let next_frame = self.current_frame.checked_sub(1);
                        if let Some(next_frame) = next_frame {
                            if tag.frames.contains(&((next_frame) as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = tag.frames.end as usize - 1;
                            }
                        } else {
                            self.current_frame = tag.frames.end as usize - 1;
                        }
                    }
                    AsepriteAnimationDirection::PingPong => {
                        if self.forward {
                            let next_frame = self.current_frame + 1;
                            if tag.frames.contains(&(next_frame as u16)) {
                                self.current_frame = next_frame;
                            } else {
                                self.current_frame = next_frame.saturating_sub(1);
                                self.forward = false;
                            }
                        } else {
                            let next_frame = self.current_frame.checked_sub(1);
                            if let Some(next_frame) = next_frame {
                                if tag.frames.contains(&(next_frame as u16)) {
                                    self.current_frame = next_frame
                                }
                            }
                            self.current_frame += 1;
                            self.forward = true;
                        }
                    }
                }
            }
            None => {
                self.current_frame = (self.current_frame + 1) % info.frame_count;
            }
        }
    }

    pub fn reset_timer(&mut self, info: &AsepriteInfo) {
        self.current_timer = self.current_frame_duration(info);
    }

    /// The current frame duration
    pub fn current_frame_duration(&self, info: &AsepriteInfo) -> f32 {
        info.frame_infos[self.current_frame].delay_ms as f32 / 1000.0
    }

    /// The current frame absolute index
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Set the current frame absolute index
    pub fn set_current_frame(&mut self, frame: usize) {
        self.current_frame = frame
    }

    /// The current frame relative index within the current tag
    pub fn current_tag_frame(&self, info: &AsepriteInfo) -> Option<usize> {
        self.tag.as_ref().and_then(|tag| {
            if let Some(tag) = info.tags.get(tag) {
                Some(self.current_frame.saturating_sub(tag.frames.start as usize))
            } else {
                error!("Tag {} wasn't found.", tag);
                None
            }
        })
    }

    /// Set current frame relative index within the current tag
    pub fn set_current_tag_frame(&mut self, info: &AsepriteInfo, frame: usize) {
        let Some(tag) = self.tag.as_ref().and_then(|tag| info.tags.get(tag)) else {
            return;
        };

        self.current_frame = (tag.frames.start as usize + frame).min(tag.frames.end as usize - 1);
    }

    /// The number of remaning frames in the current tag
    pub fn remaining_tag_frames(&self, info: &AsepriteInfo) -> Option<usize> {
        self.tag.as_ref().and_then(|tag| {
            if let Some(tag) = info.tags.get(tag) {
                Some((tag.frames.end as usize - 1) - self.current_frame)
            } else {
                error!("Tag {} wasn't found.", tag);
                None
            }
        })
    }

    /// Returns whether the current frame is finished
    pub fn frame_finished(&self, dt: f32) -> bool {
        self.current_timer <= dt
    }

    /// Starts or resumes playing an animation
    pub fn play(&mut self) {
        self.playing = true;
    }

    /// Pauses the current animation
    pub fn pause(&mut self) {
        self.playing = false;
    }

    /// Returns whether the animation is paused
    pub fn is_paused(&self) -> bool {
        !self.playing
    }

    /// Returns whether the animation is playing
    pub fn is_playing(&self) -> bool {
        !self.is_paused()
    }

    /// Toggles state between playing and pausing
    pub fn toggle(&mut self) {
        if self.is_paused() {
            self.play()
        } else {
            self.pause()
        }
    }

    pub fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }
}

pub fn update_animations(
    time: Res<Time>,
    aseprites: Res<Assets<Aseprite>>,
    mut aseprites_query: Query<(&Handle<Aseprite>, &mut AsepriteAnimation)>,
) {
    for (handle, mut animation) in aseprites_query.iter_mut() {
        let Some(aseprite) = aseprites.get(handle) else {
            error!("Aseprite handle {handle:?} is invalid");
            continue;
        };
        animation.update(aseprite.info(), time.delta_seconds());
    }
}

pub fn refresh_animations(
    mut aseprites_query: Query<(&AsepriteAnimation, &mut TextureAtlas), Changed<AsepriteAnimation>>,
) {
    for (animation, mut sprite) in aseprites_query.iter_mut() {
        sprite.index = animation.current_frame();
    }
}

/// A tag representing an animation
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsepriteTag(String);

impl From<&str> for AsepriteTag {
    fn from(tag: &str) -> Self {
        Self(tag.to_string())
    }
}

impl From<String> for AsepriteTag {
    fn from(tag: String) -> Self {
        Self(tag)
    }
}

impl std::ops::Deref for AsepriteTag {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
