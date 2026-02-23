use serde::{Deserialize, Serialize};

use crate::dotosu::sections::events;

#[derive(Serialize, Deserialize, Clone)]
pub struct Events {
    pub events: Vec<Event>,
}

impl Events {
    pub fn from_osu_format(event_section: &events::EventsSection) -> Self {
        Events {
            events: event_section
                .events
                .iter()
                .map(Event::from_osu_format)
                .collect(),
        }
    }
    pub fn to_osu_format(&self) -> events::EventsSection {
        events::EventsSection {
            events: self.events.iter().map(|e| e.to_osu_format()).collect(),
        }
    }
    pub fn background_name(&self) -> String {
        for event in &self.events {
            if let Event::Background(bg) = event {
                let file_name = bg.file_path.clone();
                if file_name.starts_with('"') && file_name.ends_with('"') {
                    return file_name[1..file_name.len() - 1].to_string();
                } else {
                    return file_name;
                }
            }
        }
        String::new()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Event {
    Background(BackgroundEvent),
    Video(VideoEvent),
    Break(BreakEvent),
    Sprite(Sprite),
    Animation(Animation),
}

impl Event {
    pub fn from_osu_format(event_line: &events::Event) -> Self {
        match event_line {
            events::Event::Background(bg) => Event::Background(BackgroundEvent {
                file_path: bg.file_path.clone(),
                start_time: bg.start_time,
                x: bg.x,
                y: bg.y,
            }),
            events::Event::Video(vd) => Event::Video(VideoEvent {
                file_path: vd.file_path.clone(),
                start_time: vd.start_time,
                x: vd.x,
                y: vd.y,
            }),
            events::Event::Break(br) => Event::Break(BreakEvent {
                start_time: br.start_time,
                end_time: br.end_time,
            }),
            events::Event::Sprite(sp) => Event::Sprite(Sprite {
                layer: sp.layer.clone(),
                origin: sp.origin.clone(),
                file_path: sp.file_path.clone(),
                x: sp.x,
                y: sp.y,
                commands: sp
                    .commands
                    .iter()
                    .cloned()
                    .map(Command::from_osu_format)
                    .collect(),
            }),
            events::Event::Animation(an) => Event::Animation(Animation {
                layer: an.layer.clone(),
                origin: an.origin.clone(),
                file_path: an.file_path.clone(),
                x: an.x,
                y: an.y,
                frame_count: an.frame_count,
                frame_delay: an.frame_delay,
                loop_type: an.loop_type.clone(),
                commands: an
                    .commands
                    .iter()
                    .cloned()
                    .map(Command::from_osu_format)
                    .collect(),
            }),
        }
    }
    pub fn to_osu_format(&self) -> events::Event {
        match self {
            Event::Background(bg) => events::Event::Background(events::BackgroundEvent {
                file_path: bg.file_path.clone(),
                start_time: bg.start_time,
                x: bg.x,
                y: bg.y,
            }),
            Event::Video(vd) => events::Event::Video(events::VideoEvent {
                file_path: vd.file_path.clone(),
                start_time: vd.start_time,
                x: vd.x,
                y: vd.y,
            }),
            Event::Break(br) => events::Event::Break(events::BreakEvent {
                start_time: br.start_time,
                end_time: br.end_time,
            }),
            Event::Sprite(sp) => events::Event::Sprite(events::Sprite {
                layer: sp.layer.clone(),
                origin: sp.origin.clone(),
                file_path: sp.file_path.clone(),
                x: sp.x,
                y: sp.y,
                commands: sp
                    .commands
                    .iter()
                    .cloned()
                    .map(|c| c.to_osu_format())
                    .collect(),
            }),
            Event::Animation(an) => events::Event::Animation(events::Animation {
                layer: an.layer.clone(),
                origin: an.origin.clone(),
                file_path: an.file_path.clone(),
                x: an.x,
                y: an.y,
                frame_count: an.frame_count,
                frame_delay: an.frame_delay,
                loop_type: an.loop_type.clone(),
                commands: an
                    .commands
                    .iter()
                    .cloned()
                    .map(|c| c.to_osu_format())
                    .collect(),
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BackgroundEvent {
    pub file_path: String,
    pub start_time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VideoEvent {
    pub file_path: String,
    pub start_time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct BreakEvent {
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Sprite {
    pub layer: String,
    pub origin: String,
    pub file_path: String,
    pub x: f64,
    pub y: f64,
    pub commands: Vec<Command>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Animation {
    pub layer: String,
    pub origin: String,
    pub file_path: String,
    pub x: f64,
    pub y: f64,
    pub frame_count: f64,
    pub frame_delay: f64,
    pub loop_type: String,
    pub commands: Vec<Command>,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Command {
    FadeCommand(FadeCommand),
    MoveCommand(MoveCommand),
    MoveXCommand(MoveXCommand),
    MoveYCommand(MoveYCommand),
    ScaleCommand(ScaleCommand),
    VectorScaleCommand(VectorScaleCommand),
    RotateCommand(RotateCommand),
    ColourCommand(ColourCommand),
    ParameterCommand(ParameterCommand),
    LoopCommand(LoopCommand),
    TriggerCommand(TriggerCommand),
}

impl Command {
    pub fn from_osu_format(command: events::Command) -> Self {
        match command {
            events::Command::FadeCommand(fc) => Command::FadeCommand(FadeCommand {
                easing: fc.easing,
                start_time: fc.start_time,
                end_time: fc.end_time,
                start_opacity: fc.start_opacity,
                end_opacity: fc.end_opacity,
            }),
            events::Command::MoveCommand(mv) => Command::MoveCommand(MoveCommand {
                easing: mv.easing,
                start_time: mv.start_time,
                end_time: mv.end_time,
                start_x: mv.start_x,
                start_y: mv.start_y,
                end_x: mv.end_x,
                end_y: mv.end_y,
            }),
            events::Command::MoveXCommand(mvx) => Command::MoveXCommand(MoveXCommand {
                easing: mvx.easing,
                start_time: mvx.start_time,
                end_time: mvx.end_time,
                start_x: mvx.start_x,
                end_x: mvx.end_x,
            }),
            events::Command::MoveYCommand(mvy) => Command::MoveYCommand(MoveYCommand {
                easing: mvy.easing,
                start_time: mvy.start_time,
                end_time: mvy.end_time,
                start_y: mvy.start_y,
                end_y: mvy.end_y,
            }),
            events::Command::ScaleCommand(sc) => Command::ScaleCommand(ScaleCommand {
                easing: sc.easing,
                start_time: sc.start_time,
                end_time: sc.end_time,
                start_scale: sc.start_scale,
                end_scale: sc.end_scale,
            }),
            events::Command::VectorScaleCommand(vsc) => {
                Command::VectorScaleCommand(VectorScaleCommand {
                    easing: vsc.easing,
                    start_time: vsc.start_time,
                    end_time: vsc.end_time,
                    start_scale_x: vsc.start_scale_x,
                    start_scale_y: vsc.start_scale_y,
                    end_scale_x: vsc.end_scale_x,
                    end_scale_y: vsc.end_scale_y,
                })
            }
            events::Command::RotateCommand(rc) => Command::RotateCommand(RotateCommand {
                easing: rc.easing,
                start_time: rc.start_time,
                end_time: rc.end_time,
                start_angle: rc.start_angle,
                end_angle: rc.end_angle,
            }),
            events::Command::ColourCommand(cc) => Command::ColourCommand(ColourCommand {
                easing: cc.easing,
                start_time: cc.start_time,
                end_time: cc.end_time,
                start_r: cc.start_r,
                start_g: cc.start_g,
                start_b: cc.start_b,
                end_r: cc.end_r,
                end_g: cc.end_g,
                end_b: cc.end_b,
            }),
            events::Command::ParameterCommand(pc) => Command::ParameterCommand(ParameterCommand {
                easing: pc.easing,
                start_time: pc.start_time,
                end_time: pc.end_time,
                parameter: match pc.parameter {
                    events::Parameter::FlipH => Parameter::FlipH,
                    events::Parameter::FlipV => Parameter::FlipV,
                    events::Parameter::AdditiveBlend => Parameter::AdditiveBlend,
                },
            }),
            events::Command::LoopCommand(lc) => Command::LoopCommand(LoopCommand {
                start_time: lc.start_time,
                loop_count: lc.loop_count,
                inner_commands: lc
                    .inner_commands
                    .into_iter()
                    .map(Command::from_osu_format)
                    .collect(),
            }),
            events::Command::TriggerCommand(tc) => Command::TriggerCommand(TriggerCommand {
                trigger_type: tc.trigger_type,
                start_time: tc.start_time,
                end_time: tc.end_time,
                inner_commands: tc
                    .inner_commands
                    .into_iter()
                    .map(Command::from_osu_format)
                    .collect(),
            }),
        }
    }
    pub fn to_osu_format(&self) -> events::Command {
        match self {
            Command::FadeCommand(fc) => events::Command::FadeCommand(events::FadeCommand {
                easing: fc.easing.clone(),
                start_time: fc.start_time,
                end_time: fc.end_time,
                start_opacity: fc.start_opacity,
                end_opacity: fc.end_opacity,
            }),
            Command::MoveCommand(mv) => events::Command::MoveCommand(events::MoveCommand {
                easing: mv.easing.clone(),
                start_time: mv.start_time,
                end_time: mv.end_time,
                start_x: mv.start_x,
                start_y: mv.start_y,
                end_x: mv.end_x,
                end_y: mv.end_y,
            }),
            Command::MoveXCommand(mvx) => events::Command::MoveXCommand(events::MoveXCommand {
                easing: mvx.easing.clone(),
                start_time: mvx.start_time,
                end_time: mvx.end_time,
                start_x: mvx.start_x,
                end_x: mvx.end_x,
            }),
            Command::MoveYCommand(mvy) => events::Command::MoveYCommand(events::MoveYCommand {
                easing: mvy.easing.clone(),
                start_time: mvy.start_time,
                end_time: mvy.end_time,
                start_y: mvy.start_y,
                end_y: mvy.end_y,
            }),
            Command::ScaleCommand(sc) => events::Command::ScaleCommand(events::ScaleCommand {
                easing: sc.easing.clone(),
                start_time: sc.start_time,
                end_time: sc.end_time,
                start_scale: sc.start_scale,
                end_scale: sc.end_scale,
            }),
            Command::VectorScaleCommand(vsc) => {
                events::Command::VectorScaleCommand(events::VectorScaleCommand {
                    easing: vsc.easing.clone(),
                    start_time: vsc.start_time,
                    end_time: vsc.end_time,
                    start_scale_x: vsc.start_scale_x,
                    start_scale_y: vsc.start_scale_y,
                    end_scale_x: vsc.end_scale_x,
                    end_scale_y: vsc.end_scale_y,
                })
            }
            Command::RotateCommand(rc) => events::Command::RotateCommand(events::RotateCommand {
                easing: rc.easing.clone(),
                start_time: rc.start_time,
                end_time: rc.end_time,
                start_angle: rc.start_angle,
                end_angle: rc.end_angle,
            }),
            Command::ColourCommand(cc) => events::Command::ColourCommand(events::ColourCommand {
                easing: cc.easing.clone(),
                start_time: cc.start_time,
                end_time: cc.end_time,
                start_r: cc.start_r,
                start_g: cc.start_g,
                start_b: cc.start_b,
                end_r: cc.end_r,
                end_g: cc.end_g,
                end_b: cc.end_b,
            }),
            Command::ParameterCommand(pc) => {
                events::Command::ParameterCommand(events::ParameterCommand {
                    easing: pc.easing.clone(),
                    start_time: pc.start_time,
                    end_time: pc.end_time,
                    parameter: match pc.parameter {
                        Parameter::FlipH => events::Parameter::FlipH,
                        Parameter::FlipV => events::Parameter::FlipV,
                        Parameter::AdditiveBlend => events::Parameter::AdditiveBlend,
                    },
                })
            }
            Command::LoopCommand(lc) => events::Command::LoopCommand(events::LoopCommand {
                start_time: lc.start_time,
                loop_count: lc.loop_count,
                inner_commands: lc
                    .inner_commands
                    .iter()
                    .cloned()
                    .map(|c| c.to_osu_format())
                    .collect(),
            }),
            Command::TriggerCommand(tc) => {
                events::Command::TriggerCommand(events::TriggerCommand {
                    trigger_type: tc.trigger_type.clone(),
                    start_time: tc.start_time,
                    end_time: tc.end_time,
                    inner_commands: tc
                        .inner_commands
                        .iter()
                        .cloned()
                        .map(|c| c.to_osu_format())
                        .collect(),
                })
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FadeCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_opacity: f64,
    pub end_opacity: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoveCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoveXCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_x: f64,
    pub end_x: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MoveYCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_y: f64,
    pub end_y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ScaleCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_scale: f64,
    pub end_scale: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct VectorScaleCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_scale_x: f64,
    pub start_scale_y: f64,
    pub end_scale_x: f64,
    pub end_scale_y: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RotateCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ColourCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_r: u8,
    pub start_g: u8,
    pub start_b: u8,
    pub end_r: u8,
    pub end_g: u8,
    pub end_b: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParameterCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub parameter: Parameter,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Parameter {
    FlipH,
    FlipV,
    AdditiveBlend,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoopCommand {
    pub start_time: f64,
    pub loop_count: i32,
    pub inner_commands: Vec<Command>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TriggerCommand {
    pub trigger_type: String,
    pub start_time: f64,
    pub end_time: f64,
    pub inner_commands: Vec<Command>,
}
