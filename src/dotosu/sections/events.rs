use std::str::FromStr;

pub struct EventsSection {
    pub events: Vec<Event>,
}

impl EventsSection {
    pub fn to_osu_text(&self) -> String {
        let mut text = String::new();
        for event in &self.events {
            match event {
                Event::Background(be) => {
                    text.push_str(&format!(
                        "Background,{},{},{},{}\n",
                        be.start_time, be.file_path, be.x, be.y
                    ));
                }
                Event::Video(ve) => {
                    text.push_str(&format!(
                        "Video,{},{},{},{}\n",
                        ve.start_time, ve.file_path, ve.x, ve.y
                    ));
                }
                Event::Break(be) => {
                    text.push_str(&format!("Break,{},{}\n", be.start_time, be.end_time));
                }
                Event::Sprite(s) => {
                    text.push_str(&format!(
                        "Sprite,{},{},{},{},{}\n",
                        s.layer, s.origin, s.file_path, s.x, s.y
                    ));
                    for command in &s.commands {
                        text.push_str(command.to_string(1).as_str());
                    }
                }
                Event::Animation(a) => {
                    text.push_str(&format!(
                        "Animation,{},{},{},{},{},{},{},{}\n",
                        a.layer,
                        a.origin,
                        a.file_path,
                        a.x,
                        a.y,
                        a.frame_count,
                        a.frame_delay,
                        a.loop_type
                    ));
                    for command in &a.commands {
                        text.push_str(command.to_string(1).as_str());
                    }
                }
            }
        }
        return text;
    }
}

pub fn parse_events_section(section_text: &str) -> Option<EventsSection> {
    let mut events: Vec<EventLine> = Vec::new();

    for line in section_text.lines() {
        let line = match line.find("//") {
            Some(index) => &line[..index],
            None => line,
        };
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() < 2 {
            println!("Invalid event line: '{}'", line);
            return None;
        }
        let tab_count = line.chars().take_while(|&c| c == ' ' || c == '_').count() as i32;
        let event_type = parts[0].trim_start_matches(|c| c == ' ' || c == '_');
        match event_type {
            "0" => {
                if parts.len() < 5 {
                    println!("Invalid Background event line: '{}'", line);
                    return None;
                }
                let start_time = match parts[1].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Background event line: '{}'", line);
                        return None;
                    }
                };
                let file_path = parts[2].trim().to_string();
                let x = match parts[3].trim().parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("Invalid x position in Background event line: '{}'", line);
                        return None;
                    }
                };
                let y = match parts[4].trim().parse::<f64>() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("Invalid y position in Background event line: '{}'", line);
                        return None;
                    }
                };

                let background = BackgroundEvent {
                    file_path,
                    start_time,
                    x,
                    y,
                };

                events.push(EventLine::Event(Event::Background(background)));
            }
            "1" | "Video" => {
                if parts.len() < 5 {
                    println!("Invalid Video event line: '{}'", line);
                    return None;
                }
                let start_time = match parts[1].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Video event line: '{}'", line);
                        return None;
                    }
                };
                let file_path = parts[2].trim().to_string();
                let x = match parts[3].trim().parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("Invalid x position in Video event line: '{}'", line);
                        return None;
                    }
                };
                let y = match parts[4].trim().parse::<f64>() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("Invalid y position in Video event line: '{}'", line);
                        return None;
                    }
                };
                let video = VideoEvent {
                    file_path,
                    start_time,
                    x,
                    y,
                };
                events.push(EventLine::Event(Event::Video(video)));
            }
            "2" | "Break" => {
                if parts.len() < 3 {
                    println!("Invalid Break event line: '{}'", line);
                    return None;
                }
                let start_time = match parts[1].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Break event line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Break event line: '{}'", line);
                        return None;
                    }
                };
                let break_event = BreakEvent {
                    start_time,
                    end_time,
                };
                events.push(EventLine::Event(Event::Break(break_event)));
            }
            "Sprite" => {
                if parts.len() < 6 {
                    println!("Invalid Sprite event line: '{}'", line);
                    return None;
                }
                let layer = parts[1].trim().to_string();
                let origin = parts[2].trim().to_string();
                let file_path = parts[3].trim().to_string();
                let x = match parts[4].trim().parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("Invalid x position in Sprite event line: '{}'", line);
                        return None;
                    }
                };
                let y = match parts[5].trim().parse::<f64>() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("Invalid y position in Sprite event line: '{}'", line);
                        return None;
                    }
                };
                let sprite = Sprite {
                    layer,
                    origin,
                    file_path,
                    x,
                    y,
                    commands: Vec::new(),
                };

                events.push(EventLine::Event(Event::Sprite(sprite)));
            }
            "Animation" => {
                if parts.len() < 9 {
                    println!("Invalid Animation event line: '{}'", line);
                    return None;
                }
                let layer = parts[1].trim().to_string();
                let origin = parts[2].trim().to_string();
                let file_path = parts[3].trim().to_string();
                let x = match parts[4].trim().parse::<f64>() {
                    Ok(x) => x,
                    Err(_) => {
                        println!("Invalid x position in Animation event line: '{}'", line);
                        return None;
                    }
                };
                let y = match parts[5].trim().parse::<f64>() {
                    Ok(y) => y,
                    Err(_) => {
                        println!("Invalid y position in Animation event line: '{}'", line);
                        return None;
                    }
                };
                let frame_count = match parts[6].trim().parse::<f64>() {
                    Ok(fc) => fc,
                    Err(_) => {
                        println!("Invalid frame count in Animation event line: '{}'", line);
                        return None;
                    }
                };
                let frame_delay = match parts[7].trim().parse::<f64>() {
                    Ok(fd) => fd,
                    Err(_) => {
                        println!("Invalid frame delay in Animation event line: '{}'", line);
                        return None;
                    }
                };
                let loop_type = parts[8].trim().to_string();
                let animation = Animation {
                    layer,
                    origin,
                    file_path,
                    x,
                    y,
                    frame_count,
                    frame_delay,
                    loop_type,
                    commands: Vec::new(),
                };

                events.push(EventLine::Event(Event::Animation(animation)));
            }
            "F" => {
                if parts.len() < 5 {
                    println!("Invalid Fade command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Fade command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Fade command line: '{}'", line);
                        return None;
                    }
                };
                let start_opacity = match parts[4].trim().parse::<f64>() {
                    Ok(o) => o,
                    Err(_) => {
                        println!("Invalid start opacity in Fade command line: '{}'", line);
                        return None;
                    }
                };
                let end_opacity = if parts.len() >= 6 {
                    match parts[5].trim().parse::<f64>() {
                        Ok(o) => o,
                        Err(_) => {
                            println!("Invalid end opacity in Fade command line: '{}'", line);
                            return None;
                        }
                    }
                } else {
                    start_opacity
                };
                let fade_command = FadeCommand {
                    easing,
                    start_time,
                    end_time,
                    start_opacity,
                    end_opacity,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::FadeCommand(fade_command),
                }));
            }
            "S" => {
                if parts.len() < 5 {
                    println!("Invalid Scale command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Scale command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Scale command line: '{}'", line);
                        return None;
                    }
                };
                let start_scale = match parts[4].trim().parse::<f64>() {
                    Ok(s) => s,
                    Err(_) => {
                        println!("Invalid start scale in Scale command line: '{}'", line);
                        return None;
                    }
                };
                let end_scale = if parts.len() >= 6 {
                    match parts[5].trim().parse::<f64>() {
                        Ok(s) => s,
                        Err(_) => {
                            println!("Invalid end scale in Scale command line: '{}'", line);
                            return None;
                        }
                    }
                } else {
                    start_scale
                };
                let scale_command = ScaleCommand {
                    easing,
                    start_time,
                    end_time,
                    start_scale,
                    end_scale,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::ScaleCommand(scale_command),
                }));
            }
            "M" => {
                if parts.len() < 8 {
                    println!("Invalid Move command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Move command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Move command line: '{}'", line);
                        return None;
                    }
                };
                let start_x = match parts[4].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_x in Move command line: '{}'", line);
                        return None;
                    }
                };
                let start_y = match parts[5].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_y in Move command line: '{}'", line);
                        return None;
                    }
                };
                let end_x = match parts[6].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid end_x in Move command line: '{}'", line);
                        return None;
                    }
                };
                let end_y = match parts[7].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid end_y in Move command line: '{}'", line);
                        return None;
                    }
                };
                let cmd = MoveCommand {
                    easing,
                    start_time,
                    end_time,
                    start_x,
                    start_y,
                    end_x,
                    end_y,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::MoveCommand(cmd),
                }));
            }
            "MX" => {
                if parts.len() < 5 {
                    println!("Invalid MoveX command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in MoveX command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in MoveX command line: '{}'", line);
                        return None;
                    }
                };
                let start_x = match parts[4].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_x in MoveX command line: '{}'", line);
                        return None;
                    }
                };
                let end_x = if parts.len() >= 6 {
                    match parts[5].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_x in MoveX command line: '{}'", line);
                            return None;
                        }
                    }
                } else {
                    start_x
                };
                let cmd = MoveXCommand {
                    easing,
                    start_time,
                    end_time,
                    start_x,
                    end_x,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::MoveXCommand(cmd),
                }));
            }
            "MY" => {
                if parts.len() < 5 {
                    println!("Invalid MoveY command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in MoveY command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in MoveY command line: '{}'", line);
                        return None;
                    }
                };
                let start_y = match parts[4].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_y in MoveY command line: '{}'", line);
                        return None;
                    }
                };
                let end_y = if parts.len() >= 6 {
                    match parts[5].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_y in MoveY command line: '{}'", line);
                            return None;
                        }
                    }
                } else {
                    start_y
                };
                let cmd = MoveYCommand {
                    easing,
                    start_time,
                    end_time,
                    start_y,
                    end_y,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::MoveYCommand(cmd),
                }));
            }
            "V" => {
                if parts.len() < 6 {
                    println!("Invalid VectorScale command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in VectorScale command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in VectorScale command line: '{}'", line);
                        return None;
                    }
                };
                let start_scale_x = match parts[4].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!(
                            "Invalid start_scale_x in VectorScale command line: '{}'",
                            line
                        );
                        return None;
                    }
                };
                let start_scale_y = match parts[5].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!(
                            "Invalid start_scale_y in VectorScale command line: '{}'",
                            line
                        );
                        return None;
                    }
                };
                let (end_scale_x, end_scale_y) = if parts.len() >= 8 {
                    let ex = match parts[6].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!(
                                "Invalid end_scale_x in VectorScale command line: '{}'",
                                line
                            );
                            return None;
                        }
                    };
                    let ey = match parts[7].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!(
                                "Invalid end_scale_y in VectorScale command line: '{}'",
                                line
                            );
                            return None;
                        }
                    };
                    (ex, ey)
                } else {
                    (start_scale_x, start_scale_y)
                };
                let cmd = VectorScaleCommand {
                    easing,
                    start_time,
                    end_time,
                    start_scale_x,
                    start_scale_y,
                    end_scale_x,
                    end_scale_y,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::VectorScaleCommand(cmd),
                }));
            }
            "R" => {
                if parts.len() < 5 {
                    println!("Invalid Rotate command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Rotate command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Rotate command line: '{}'", line);
                        return None;
                    }
                };
                let start_angle = match parts[4].trim().parse::<f64>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_angle in Rotate command line: '{}'", line);
                        return None;
                    }
                };
                let end_angle = if parts.len() >= 6 {
                    match parts[5].trim().parse::<f64>() {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_angle in Rotate command line: '{}'", line);
                            return None;
                        }
                    }
                } else {
                    start_angle
                };
                let cmd = RotateCommand {
                    easing,
                    start_time,
                    end_time,
                    start_angle,
                    end_angle,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::RotateCommand(cmd),
                }));
            }
            "C" => {
                if parts.len() < 7 {
                    println!("Invalid Colour command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Colour command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Colour command line: '{}'", line);
                        return None;
                    }
                };

                let parse_u8 =
                    |s: &str| -> Result<u8, <u8 as FromStr>::Err> { s.trim().parse::<u8>() };
                let start_r = match parse_u8(parts[4]) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_r in Colour command line: '{}'", line);
                        return None;
                    }
                };
                let start_g = match parse_u8(parts[5]) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_g in Colour command line: '{}'", line);
                        return None;
                    }
                };
                let start_b = match parse_u8(parts[6]) {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid start_b in Colour command line: '{}'", line);
                        return None;
                    }
                };
                let (end_r, end_g, end_b) = if parts.len() >= 10 {
                    let er = match parse_u8(parts[7]) {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_r in Colour command line: '{}'", line);
                            return None;
                        }
                    };
                    let eg = match parse_u8(parts[8]) {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_g in Colour command line: '{}'", line);
                            return None;
                        }
                    };
                    let eb = match parse_u8(parts[9]) {
                        Ok(v) => v,
                        Err(_) => {
                            println!("Invalid end_b in Colour command line: '{}'", line);
                            return None;
                        }
                    };
                    (er, eg, eb)
                } else {
                    (start_r, start_g, start_b)
                };

                let cmd = ColourCommand {
                    easing,
                    start_time,
                    end_time,
                    start_r,
                    start_g,
                    start_b,
                    end_r,
                    end_g,
                    end_b,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::ColourCommand(cmd),
                }));
            }
            "P" => {
                if parts.len() < 5 {
                    println!("Invalid Parameter command line: '{}'", line);
                    return None;
                }
                let easing = parts[1].trim().to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Parameter command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Parameter command line: '{}'", line);
                        return None;
                    }
                };
                let param_str = parts[4].trim().trim_matches('"');
                let parameter = match param_str {
                    "H" => Parameter::FlipH,
                    "V" => Parameter::FlipV,
                    "A" => Parameter::AdditiveBlend,
                    _ => {
                        println!(
                            "Invalid parameter '{}' in Parameter command line: '{}'",
                            param_str, line
                        );
                        return None;
                    }
                };
                let cmd = ParameterCommand {
                    easing,
                    start_time,
                    end_time,
                    parameter,
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::ParameterCommand(cmd),
                }));
            }
            "L" => {
                if parts.len() < 3 {
                    println!("Invalid Loop command line: '{}'", line);
                    return None;
                }
                let start_time = match parts[1].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Loop command line: '{}'", line);
                        return None;
                    }
                };
                let loop_count = match parts[2].trim().parse::<i32>() {
                    Ok(v) => v,
                    Err(_) => {
                        println!("Invalid loop_count in Loop command line: '{}'", line);
                        return None;
                    }
                };
                let cmd = LoopCommand {
                    start_time,
                    loop_count,
                    inner_commands: Vec::new(),
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::LoopCommand(cmd),
                }));
            }
            "T" => {
                if parts.len() < 4 {
                    println!("Invalid Trigger command line: '{}'", line);
                    return None;
                }
                let trigger_type = parts[1].trim().trim_matches('"').to_string();
                let start_time = match parts[2].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid start time in Trigger command line: '{}'", line);
                        return None;
                    }
                };
                let end_time = match parts[3].trim().parse::<f64>() {
                    Ok(t) => t,
                    Err(_) => {
                        println!("Invalid end time in Trigger command line: '{}'", line);
                        return None;
                    }
                };
                let cmd = TriggerCommand {
                    trigger_type,
                    start_time,
                    end_time,
                    inner_commands: Vec::new(),
                };
                events.push(EventLine::Command(CommandLineWithTab {
                    tab_count,
                    command: Command::TriggerCommand(cmd),
                }));
            }
            _ => {
                println!(
                    "Unsupported event type '{}' in line: '{}'",
                    event_type, line
                );
                return None;
            }
        }
    }
    match collect_events(&events) {
        None => {
            println!("Failed to collect events from parsed lines.");
            return None;
        }
        Some(collected_events) => {
            return Some(EventsSection {
                events: collected_events,
            });
        }
    }
}

fn collect_events(events_with_tabs: &Vec<EventLine>) -> Option<Vec<Event>> {
    let mut events: Vec<Event> = Vec::new();
    let mut commands: Vec<CommandLineWithTab> = Vec::new();

    let mut i = events_with_tabs.len() as i32 - 1;
    while i >= 0 {
        match &events_with_tabs[i as usize].clone() {
            EventLine::Event(e) => {
                commands.reverse();
                let collected = collect_commands(commands.clone().as_slice());
                commands = Vec::new();
                match e {
                    Event::Sprite(s) => {
                        events.push(Event::Sprite(Sprite {
                            layer: s.layer.clone(),
                            origin: s.origin.clone(),
                            file_path: s.file_path.clone(),
                            x: s.x,
                            y: s.y,
                            commands: collected,
                        }));
                    }
                    Event::Animation(a) => {
                        events.push(Event::Animation(Animation {
                            layer: a.layer.clone(),
                            origin: a.origin.clone(),
                            file_path: a.file_path.clone(),
                            x: a.x,
                            y: a.y,
                            frame_count: a.frame_count,
                            frame_delay: a.frame_delay,
                            loop_type: a.loop_type.clone(),
                            commands: collected,
                        }));
                    }
                    other_event => {
                        events.push(other_event.clone());
                    }
                }
            }
            EventLine::Command(cwt) => {
                commands.push(cwt.clone());
            }
        }
        i -= 1;
    }
    events.reverse();
    return Some(events);
}

fn collect_commands(commands_with_tabs: &[CommandLineWithTab]) -> Vec<Command> {
    if commands_with_tabs.is_empty() {
        return Vec::new();
    }
    let mut commands: Vec<Command> = Vec::new();
    let mut i = 0;
    while i < commands_with_tabs.len() {
        let current_tab_count = commands_with_tabs[i].tab_count;
        let mut j = i + 1;
        while j < commands_with_tabs.len() && commands_with_tabs[j].tab_count > current_tab_count {
            j += 1;
        }
        let inner_commands = collect_commands(&commands_with_tabs[i + 1..j]);
        let command = commands_with_tabs[i].command.clone();
        i = j;

        match command {
            Command::LoopCommand(mut loop_cmd) => {
                loop_cmd.inner_commands = inner_commands;
                commands.push(Command::LoopCommand(loop_cmd));
            }
            Command::TriggerCommand(mut trigger_cmd) => {
                trigger_cmd.inner_commands = inner_commands;
                commands.push(Command::TriggerCommand(trigger_cmd));
            }
            _ => {
                commands.push(command);
            }
        }
    }
    return commands;
}

#[derive(Clone)]
struct CommandLineWithTab {
    tab_count: i32,
    command: Command,
}

#[derive(Clone)]
enum EventLine {
    Event(Event),
    Command(CommandLineWithTab),
}

#[derive(Clone)]
pub enum Event {
    Background(BackgroundEvent),
    Video(VideoEvent),
    Break(BreakEvent),
    Sprite(Sprite),
    Animation(Animation),
}

#[derive(Clone)]
pub struct BackgroundEvent {
    pub file_path: String,
    pub start_time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
pub struct VideoEvent {
    pub file_path: String,
    pub start_time: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone)]
pub struct BreakEvent {
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Clone)]
pub struct Sprite {
    pub layer: String,
    pub origin: String,
    pub file_path: String,
    pub x: f64,
    pub y: f64,
    pub commands: Vec<Command>,
}

#[derive(Clone)]
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

#[derive(Clone)]
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
    pub fn to_string(&self, tab_count: i32) -> String {
        let mut result = String::new();
        let tabs = "_".repeat(tab_count as usize);
        match self {
            Command::FadeCommand(fc) => {
                result.push_str(&format!(
                    "{}F,{},{},{},{},{}\n",
                    tabs, fc.easing, fc.start_time, fc.end_time, fc.start_opacity, fc.end_opacity
                ));
            }
            Command::MoveCommand(mc) => {
                result.push_str(&format!(
                    "{}M,{},{},{},{},{},{},{}\n",
                    tabs,
                    mc.easing,
                    mc.start_time,
                    mc.end_time,
                    mc.start_x,
                    mc.start_y,
                    mc.end_x,
                    mc.end_y
                ));
            }
            Command::MoveXCommand(mc) => {
                result.push_str(&format!(
                    "{}MX,{},{},{},{},{}\n",
                    tabs, mc.easing, mc.start_time, mc.end_time, mc.start_x, mc.end_x
                ));
            }
            Command::MoveYCommand(mc) => {
                result.push_str(&format!(
                    "{}MY,{},{},{},{},{}\n",
                    tabs, mc.easing, mc.start_time, mc.end_time, mc.start_y, mc.end_y
                ));
            }
            Command::ScaleCommand(sc) => {
                result.push_str(&format!(
                    "{}S,{},{},{},{},{}\n",
                    tabs, sc.easing, sc.start_time, sc.end_time, sc.start_scale, sc.end_scale
                ));
            }
            Command::VectorScaleCommand(sc) => {
                result.push_str(&format!(
                    "{}V,{},{},{},{},{},{},{}\n",
                    tabs,
                    sc.easing,
                    sc.start_time,
                    sc.end_time,
                    sc.start_scale_x,
                    sc.start_scale_y,
                    sc.end_scale_x,
                    sc.end_scale_y
                ));
            }
            Command::RotateCommand(rc) => {
                result.push_str(&format!(
                    "{}R,{},{},{},{},{}\n",
                    tabs, rc.easing, rc.start_time, rc.end_time, rc.start_angle, rc.end_angle
                ));
            }
            Command::ColourCommand(cc) => {
                result.push_str(&format!(
                    "{}C,{},{},{},{},{},{},{},{},{}\n",
                    tabs,
                    cc.easing,
                    cc.start_time,
                    cc.end_time,
                    cc.start_r,
                    cc.start_g,
                    cc.start_b,
                    cc.end_r,
                    cc.end_g,
                    cc.end_b
                ));
            }
            Command::ParameterCommand(pc) => {
                let p = match pc.parameter {
                    Parameter::FlipH => "H",
                    Parameter::FlipV => "V",
                    Parameter::AdditiveBlend => "A",
                };
                result.push_str(&format!(
                    "{}P,{},{},{},{}\n",
                    tabs, pc.easing, pc.start_time, pc.end_time, p
                ));
            }
            Command::LoopCommand(lc) => {
                result.push_str(&format!("{}L,{},{}\n", tabs, lc.start_time, lc.loop_count));
            }
            Command::TriggerCommand(tc) => {
                result.push_str(&format!(
                    "{}T,{},{},{}\n",
                    tabs, tc.trigger_type, tc.start_time, tc.end_time
                ));
            }
        }
        match self {
            Command::LoopCommand(loop_cmd) => {
                for inner_command in &loop_cmd.inner_commands {
                    result.push_str(&inner_command.to_string(tab_count + 1));
                }
            }
            Command::TriggerCommand(trigger_cmd) => {
                for inner_command in &trigger_cmd.inner_commands {
                    result.push_str(&inner_command.to_string(tab_count + 1));
                }
            }
            _ => {}
        }
        return result;
    }
}

#[derive(Clone)]
pub struct FadeCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_opacity: f64,
    pub end_opacity: f64,
}

#[derive(Clone)]
pub struct MoveCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_x: f64,
    pub start_y: f64,
    pub end_x: f64,
    pub end_y: f64,
}

#[derive(Clone)]
pub struct MoveXCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_x: f64,
    pub end_x: f64,
}

#[derive(Clone)]
pub struct MoveYCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_y: f64,
    pub end_y: f64,
}

#[derive(Clone)]
pub struct ScaleCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_scale: f64,
    pub end_scale: f64,
}

#[derive(Clone)]
pub struct VectorScaleCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_scale_x: f64,
    pub start_scale_y: f64,
    pub end_scale_x: f64,
    pub end_scale_y: f64,
}

#[derive(Clone)]
pub struct RotateCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

#[derive(Clone)]
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

#[derive(Clone)]
pub struct ParameterCommand {
    pub easing: String,
    pub start_time: f64,
    pub end_time: f64,
    pub parameter: Parameter,
}

#[derive(Clone)]
pub enum Parameter {
    FlipH,
    FlipV,
    AdditiveBlend,
}

#[derive(Clone)]
pub struct LoopCommand {
    pub start_time: f64,
    pub loop_count: i32,
    pub inner_commands: Vec<Command>,
}

#[derive(Clone)]
pub struct TriggerCommand {
    pub trigger_type: String,
    pub start_time: f64,
    pub end_time: f64,
    pub inner_commands: Vec<Command>,
}
