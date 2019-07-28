//! # Implementing new commands.
//! To implement a new command, add an instance of `ChatCommand` to `CHAT_COMMANDS`
//! and provide a handler function.

use crate::Server;
use chrono::{NaiveTime, Timelike};
use common::{
    comp,
    msg::ServerMsg,
    npc::{get_npc_name, NpcKind},
    state::TimeOfDay,
};
use rand::Rng;
use specs::{Builder, Entity as EcsEntity, Join};
use vek::*;

use lazy_static::lazy_static;
use scan_fmt::scan_fmt;

/// Struct representing a command that a user can run from server chat.
pub struct ChatCommand {
    /// The keyword used to invoke the command, omitting the leading '/'.
    pub keyword: &'static str,
    /// A format string for parsing arguments.
    arg_fmt: &'static str,
    /// A message that explains how the command is used.
    help_string: &'static str,
    /// Handler function called when the command is executed.
    /// # Arguments
    /// * `&mut Server` - the `Server` instance executing the command.
    /// * `EcsEntity` - an `Entity` corresponding to the player that invoked the command.
    /// * `String` - a `String` containing the part of the command after the keyword.
    /// * `&ChatCommand` - the command to execute with the above arguments.
    /// Handler functions must parse arguments from the the given `String` (`scan_fmt!` is included for this purpose).
    handler: fn(&mut Server, EcsEntity, String, &ChatCommand),
}

impl ChatCommand {
    /// Creates a new chat command.
    pub fn new(
        keyword: &'static str,
        arg_fmt: &'static str,
        help_string: &'static str,
        handler: fn(&mut Server, EcsEntity, String, &ChatCommand),
    ) -> Self {
        Self {
            keyword,
            arg_fmt,
            help_string,
            handler,
        }
    }
    /// Calls the contained handler function, passing `&self` as the last argument.
    pub fn execute(&self, server: &mut Server, entity: EcsEntity, args: String) {
        (self.handler)(server, entity, args, self);
    }
}

lazy_static! {
    /// Static list of chat commands available to the server.
    pub static ref CHAT_COMMANDS: Vec<ChatCommand> = vec![
        ChatCommand::new(
            "jump",
            "{d} {d} {d}",
            "/jump <dx> <dy> <dz> : Offset your current position",
            handle_jump,
        ),
        ChatCommand::new(
            "goto",
            "{d} {d} {d}",
            "/goto <x> <y> <z> : Teleport to a position",
            handle_goto,
        ),
        ChatCommand::new(
            "alias",
            "{}",
            "/alias <name> : Change your alias",
            handle_alias,
        ),
        ChatCommand::new(
            "tp",
            "{}",
            "/tp <alias> : Teleport to another player",
            handle_tp,
        ),
        ChatCommand::new(
            "kill",
            "{}",
            "/kill : Kill yourself",
            handle_kill,
        ),
        ChatCommand::new(
            "time",
            "{} {s}",
            "/time : Set the time of day",
            handle_time,
        ),
        ChatCommand::new(
            "spawn",
            "{} {} {d}",
            "/spawn <alignment> <entity> [amount] : Spawn a test entity",
            handle_spawn,
        ),
        ChatCommand::new(
             "players",
             "{}",
             "/players : Show the online players list",
             handle_players,
         ),
        ChatCommand::new(
            "help", "", "/help: Display this message", handle_help),
        ChatCommand::new(
            "health",
            "{}",
            "/health : Set your current health",
            handle_health,
        ),
        ChatCommand::new(
            "build",
            "",
            "/build : Toggles build mode on and off",
            handle_build,
        ),
        ChatCommand::new(
            "tell",
            "{}",
            "/tell <alias> <message>: Send a message to another player",
            handle_tell,
        ),
        ChatCommand::new(
            "killnpcs",
            "{}",
            "/killnpcs : Kill the NPCs",
            handle_killnpcs,
        ),
        ChatCommand::new(
            "object",
            "{}",
            "/object [Name]: Spawn an object",
            handle_object,
        ),
        ChatCommand::new(
            "light",
            "{} {} {} {} {} {} {}",
            "/light <opt:  <<cr> <cg> <cb>> <<ox> <oy> <oz>> <<strenght>>>: Spawn entity with light",
            handle_light,
        ),
        ChatCommand::new(
            "lantern",
            "{} ",
            "/lantern : adds/remove light near player",
            handle_lantern,
        ),
    ];
}

fn handle_jump(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
    match (opt_x, opt_y, opt_z) {
        (Some(x), Some(y), Some(z)) => {
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(current_pos) => {
                    server
                        .state
                        .write_component(entity, comp::Pos(current_pos.0 + Vec3::new(x, y, z)));
                    server.state.write_component(entity, comp::ForceUpdate);
                }
                None => server.clients.notify(
                    entity,
                    ServerMsg::private(String::from("You have no position!")),
                ),
            }
        }
        _ => server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string))),
    }
}

fn handle_goto(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_x, opt_y, opt_z) = scan_fmt!(&args, action.arg_fmt, f32, f32, f32);
    match server.state.read_component_cloned::<comp::Pos>(entity) {
        Some(_pos) => match (opt_x, opt_y, opt_z) {
            (Some(x), Some(y), Some(z)) => {
                server
                    .state
                    .write_component(entity, comp::Pos(Vec3::new(x, y, z)));
                server.state.write_component(entity, comp::ForceUpdate);
            }
            _ => server
                .clients
                .notify(entity, ServerMsg::private(String::from(action.help_string))),
        },
        None => {
            server.clients.notify(
                entity,
                ServerMsg::private(String::from("You don't have any position!")),
            );
        }
    }
}

fn handle_kill(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
        .map(|s| s.health.set_to(0, comp::HealthSource::Suicide));
}

fn handle_time(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let time = scan_fmt!(&args, action.arg_fmt, String);
    let new_time = match time.as_ref().map(|s| s.as_str()) {
        Some("night") => NaiveTime::from_hms(0, 0, 0),
        Some("dawn") => NaiveTime::from_hms(5, 0, 0),
        Some("day") => NaiveTime::from_hms(12, 0, 0),
        Some("dusk") => NaiveTime::from_hms(17, 0, 0),
        Some(n) => match n.parse() {
            Ok(n) => n,
            Err(_) => match NaiveTime::parse_from_str(n, "%H:%M") {
                Ok(time) => time,
                Err(_) => {
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("'{}' is not a valid time.", n)),
                    );
                    return;
                }
            },
        },
        None => {
            let time_in_seconds = server.state.ecs_mut().read_resource::<TimeOfDay>().0;
            let current_time = NaiveTime::from_num_seconds_from_midnight(time_in_seconds as u32, 0);
            server.clients.notify(
                entity,
                ServerMsg::private(format!(
                    "Current time is: {}",
                    current_time.format("%H:%M").to_string()
                )),
            );
            return;
        }
    };

    server.state.ecs_mut().write_resource::<TimeOfDay>().0 =
        new_time.num_seconds_from_midnight() as f64;

    server.clients.notify(
        entity,
        ServerMsg::private(format!(
            "Time changed to: {}",
            new_time.format("%H:%M").to_string()
        )),
    );
}

fn handle_health(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_hp = scan_fmt!(&args, action.arg_fmt, u32);

    match server
        .state
        .ecs_mut()
        .write_storage::<comp::Stats>()
        .get_mut(entity)
    {
        Some(stats) => match opt_hp {
            Some(hp) => stats.health.set_to(hp, comp::HealthSource::Command),
            None => {
                server.clients.notify(
                    entity,
                    ServerMsg::private(String::from("You must specify health amount!")),
                );
            }
        },
        None => server.clients.notify(
            entity,
            ServerMsg::private(String::from("You have no position.")),
        ),
    }
}

fn handle_alias(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            server
                .state
                .ecs_mut()
                .write_storage::<comp::Player>()
                .get_mut(entity)
                .map(|player| player.alias = alias);
        }
        None => server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string))),
    }
}

fn handle_tp(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            let ecs = server.state.ecs();
            let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == alias)
                .map(|(entity, _)| entity);
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(_pos) => match opt_player {
                    Some(player) => match server.state.read_component_cloned::<comp::Pos>(player) {
                        Some(pos) => {
                            server.state.write_component(entity, pos);
                            server.state.write_component(entity, comp::ForceUpdate);
                        }
                        None => server.clients.notify(
                            entity,
                            ServerMsg::private(format!(
                                "Unable to teleport to player '{}'!",
                                alias
                            )),
                        ),
                    },
                    None => {
                        server.clients.notify(
                            entity,
                            ServerMsg::private(format!("Player '{}' not found!", alias)),
                        );
                        server
                            .clients
                            .notify(entity, ServerMsg::private(String::from(action.help_string)));
                    }
                },
                None => {
                    server
                        .clients
                        .notify(entity, ServerMsg::private(format!("You have no position!")));
                }
            }
        }
        None => server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string))),
    }
}

fn handle_spawn(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_align, opt_id, opt_amount) = scan_fmt!(&args, action.arg_fmt, String, NpcKind, String);
    // This should be just an enum handled with scan_fmt!
    let opt_agent = alignment_to_agent(&opt_align.unwrap_or(String::new()), entity);
    let _objtype = scan_fmt!(&args, action.arg_fmt, String);
    // Make sure the amount is either not provided or a valid value
    let opt_amount = opt_amount
        .map_or(Some(1), |a| a.parse().ok())
        .and_then(|a| if a > 0 { Some(a) } else { None });

    match (opt_agent, opt_id, opt_amount) {
        (Some(agent), Some(id), Some(amount)) => {
            match server.state.read_component_cloned::<comp::Pos>(entity) {
                Some(pos) => {
                    for _ in 0..amount {
                        let vel = Vec3::new(
                            rand::thread_rng().gen_range(-2.0, 3.0),
                            rand::thread_rng().gen_range(-2.0, 3.0),
                            10.0,
                        );

                        let body = kind_to_body(id);
                        server
                            .create_npc(pos, get_npc_name(id), body)
                            .with(comp::Vel(vel))
                            .with(agent)
                            .build();
                    }
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("Spawned {} entities", amount).to_owned()),
                    );
                }
                None => server.clients.notify(
                    entity,
                    ServerMsg::private("You have no position!".to_owned()),
                ),
            }
        }
        _ => server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string))),
    }
}

fn handle_players(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    let ecs = server.state.ecs();
    let players = ecs.read_storage::<comp::Player>();
    let count = players.join().count();
    let header_message: String = format!("{} online players: \n", count);
    if count > 0 {
        let mut player_iter = players.join();
        let first = player_iter.next().unwrap().alias.to_owned();
        let player_list = player_iter.fold(first, |mut s, p| {
            s += ",\n";
            s += &p.alias;
            s
        });

        server
            .clients
            .notify(entity, ServerMsg::private(header_message + &player_list));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::private(header_message));
    }
}

fn handle_build(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    if server
        .state
        .read_storage::<comp::CanBuild>()
        .get(entity)
        .is_some()
    {
        server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .remove(entity);
        server.clients.notify(
            entity,
            ServerMsg::private(String::from("Toggled off build mode!")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::CanBuild>()
            .insert(entity, comp::CanBuild);
        server.clients.notify(
            entity,
            ServerMsg::private(String::from("Toggled on build mode!")),
        );
    }
}

fn handle_help(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    for cmd in CHAT_COMMANDS.iter() {
        server
            .clients
            .notify(entity, ServerMsg::private(String::from(cmd.help_string)));
    }
}

fn alignment_to_agent(alignment: &str, target: EcsEntity) -> Option<comp::Agent> {
    match alignment {
        "hostile" => Some(comp::Agent::Enemy { target: None }),
        "friendly" => Some(comp::Agent::Pet {
            target,
            offset: Vec2::zero(),
        }),
        // passive?
        _ => None,
    }
}

fn kind_to_body(kind: NpcKind) -> comp::Body {
    match kind {
        NpcKind::Humanoid => comp::Body::Humanoid(comp::humanoid::Body::random()),
        NpcKind::Pig => comp::Body::Quadruped(comp::quadruped::Body::random()),
        NpcKind::Wolf => comp::Body::QuadrupedMedium(comp::quadruped_medium::Body::random()),
    }
}

fn handle_killnpcs(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    let ecs = server.state.ecs();
    let mut stats = ecs.write_storage::<comp::Stats>();
    let players = ecs.read_storage::<comp::Player>();
    let mut count = 0;
    for (stats, ()) in (&mut stats, !&players).join() {
        count += 1;
        stats.health.set_to(0, comp::HealthSource::Command);
    }
    let text = if count > 0 {
        format!("Destroyed {} NPCs.", count)
    } else {
        "No NPCs on server.".to_string()
    };
    server.clients.notify(entity, ServerMsg::private(text));
}

fn handle_object(server: &mut Server, entity: EcsEntity, args: String, _action: &ChatCommand) {
    let obj_type = scan_fmt!(&args, _action.arg_fmt, String);

    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(entity)
        .copied();
    let ori = server
        .state
        .ecs()
        .read_storage::<comp::Ori>()
        .get(entity)
        .copied();
    /*let builder = server
    .create_object(pos, ori, obj_type)
    .with(ori);*/
    if let (Some(pos), Some(ori)) = (pos, ori) {
        let obj_str_opt = obj_type.as_ref().map(String::as_str);
        let obj_type = match obj_str_opt {
            Some("scarecrow") => comp::object::Body::Scarecrow,
            Some("cauldron") => comp::object::Body::Cauldron,
            Some("chest_vines") => comp::object::Body::ChestVines,
            Some("chest") => comp::object::Body::Chest,
            Some("chest_dark") => comp::object::Body::ChestDark,
            Some("chest_demon") => comp::object::Body::ChestDemon,
            Some("chest_gold") => comp::object::Body::ChestGold,
            Some("chest_light") => comp::object::Body::ChestLight,
            Some("chest_open") => comp::object::Body::ChestOpen,
            Some("chest_skull") => comp::object::Body::ChestSkull,
            Some("pumpkin") => comp::object::Body::Pumpkin,
            Some("pumpkin_2") => comp::object::Body::Pumpkin2,
            Some("pumpkin_3") => comp::object::Body::Pumpkin3,
            Some("pumpkin_4") => comp::object::Body::Pumpkin4,
            Some("pumpkin_5") => comp::object::Body::Pumpkin5,
            Some("campfire") => comp::object::Body::Campfire,
            Some("lantern_ground") => comp::object::Body::LanternGround,
            Some("lantern_ground_open") => comp::object::Body::LanternGroundOpen,
            Some("lantern_2") => comp::object::Body::LanternStanding2,
            Some("lantern") => comp::object::Body::LanternStanding,
            Some("potion_blue") => comp::object::Body::PotionBlue,
            Some("potion_green") => comp::object::Body::PotionGreen,
            Some("potion_red") => comp::object::Body::PotionRed,
            Some("crate") => comp::object::Body::Crate,
            Some("tent") => comp::object::Body::Tent,
            Some("bomb") => comp::object::Body::Bomb,
            Some("window_spooky") => comp::object::Body::WindowSpooky,
            Some("door_spooky") => comp::object::Body::DoorSpooky,
            Some("carpet") => comp::object::Body::Carpet,
            Some("table_human") => comp::object::Body::Table,
            Some("table_human_2") => comp::object::Body::Table2,
            Some("table_human_3") => comp::object::Body::Table3,
            Some("drawer") => comp::object::Body::Drawer,
            Some("bed_human_blue") => comp::object::Body::BedBlue,
            Some("anvil") => comp::object::Body::Anvil,
            Some("gravestone") => comp::object::Body::Gravestone,
            Some("gravestone_2") => comp::object::Body::Gravestone2,
            Some("chair") => comp::object::Body::Chair,
            Some("chair_2") => comp::object::Body::Chair2,
            Some("chair_3") => comp::object::Body::Chair3,
            Some("bench_human") => comp::object::Body::Bench,
            Some("bedroll") => comp::object::Body::Bedroll,
            Some("carpet_human_round") => comp::object::Body::CarpetHumanRound,
            Some("carpet_human_square") => comp::object::Body::CarpetHumanSquare,
            Some("carpet_human_square_2") => comp::object::Body::CarpetHumanSquare2,
            Some("carpet_human_squircle") => comp::object::Body::CarpetHumanSquircle,
            _ => {
                return server
                    .clients
                    .notify(entity, ServerMsg::chat(String::from("Object not found!")));
            }
        };
        server
            .create_object(pos, ori, obj_type)
            .with(comp::Ori(
                // converts player orientation into a 90° rotation for the object by using the axis with the highest value
                ori.0
                    .map(|e| {
                        if e.abs() == ori.0.map(|e| e.abs()).reduce_partial_max() {
                            e
                        } else {
                            0.0
                        }
                    })
                    .normalized(),
            ))
            .build();
        server.clients.notify(
            entity,
            ServerMsg::chat(format!("Spawned object: {}", obj_str_opt.unwrap())),
        );
    } else {
        server
            .clients
            .notify(entity, ServerMsg::chat(format!("You have no position!")));
    }
}

fn handle_light(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let (opt_r, opt_g, opt_b, opt_x, opt_y, opt_z, opt_s) =
        scan_fmt!(&args, action.arg_fmt, f32, f32, f32, f32, f32, f32, f32);

    let mut light_emitter = comp::LightEmitter::default();

    if let (Some(r), Some(g), Some(b)) = (opt_r, opt_g, opt_b) {
        light_emitter.col = Rgb::new(r, g, b)
    };
    if let (Some(x), Some(y), Some(z)) = (opt_x, opt_y, opt_z) {
        light_emitter.offset = Vec3::new(x, y, z)
    };
    if let Some(s) = opt_s {
        light_emitter.strength = s
    };
    let pos = server
        .state
        .ecs()
        .read_storage::<comp::Pos>()
        .get(entity)
        .copied();
    if let Some(pos) = pos {
        server
            .state
            .ecs_mut()
            .create_entity_synced()
            .with(pos)
            .with(comp::ForceUpdate)
            .with(light_emitter)
            .build();
        server
            .clients
            .notify(entity, ServerMsg::chat(format!("Spawned object.")));
    } else {
        server
            .clients
            .notify(entity, ServerMsg::chat(format!("You have no position!")));
    }
}

fn handle_lantern(server: &mut Server, entity: EcsEntity, _args: String, _action: &ChatCommand) {
    if server
        .state
        .read_storage::<comp::LightEmitter>()
        .get(entity)
        .is_some()
    {
        server
            .state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .remove(entity);
        server.clients.notify(
            entity,
            ServerMsg::chat(String::from("You put out the lantern.")),
        );
    } else {
        let _ = server
            .state
            .ecs()
            .write_storage::<comp::LightEmitter>()
            .insert(
                entity,
                comp::LightEmitter {
                    offset: Vec3::new(1.0, 0.2, 0.8),
                    col: Rgb::new(0.824, 0.365, 0.196),
                    strength: 1.5,
                },
            );

        server.clients.notify(
            entity,
            ServerMsg::chat(String::from("You lighted your lantern.")),
        );
    }
}

fn handle_tell(server: &mut Server, entity: EcsEntity, args: String, action: &ChatCommand) {
    let opt_alias = scan_fmt!(&args, action.arg_fmt, String);
    match opt_alias {
        Some(alias) => {
            let ecs = server.state.ecs();
            let opt_player = (&ecs.entities(), &ecs.read_storage::<comp::Player>())
                .join()
                .find(|(_, player)| player.alias == alias)
                .map(|(entity, _)| entity);
            let msg = &args[alias.len()..args.len()];
            match opt_player {
                Some(player) => {
                    if player != entity {
                        if msg.len() > 1 {
                            let opt_name = ecs
                                .read_storage::<comp::Player>()
                                .get(entity)
                                .map(|s| s.alias.clone());
                            match opt_name {
                                Some(name) => {
                                    server.clients.notify(
                                        player,
                                        ServerMsg::tell(format!("{} tells you:{}", name, msg)),
                                    );
                                    server.clients.notify(
                                        entity,
                                        ServerMsg::tell(format!("You tell {}:{}", alias, msg)),
                                    );
                                }
                                None => {
                                    server.clients.notify(
                                        entity,
                                        ServerMsg::private(String::from("You do not exist!")),
                                    );
                                }
                            }
                        } else {
                            server.clients.notify(
                                entity,
                                ServerMsg::private(format!(
                                    "You really should say something to {}!",
                                    alias
                                )),
                            );
                        }
                    } else {
                        server.clients.notify(
                            entity,
                            ServerMsg::private(format!("You can't /tell yourself.")),
                        );
                    }
                }
                None => {
                    server.clients.notify(
                        entity,
                        ServerMsg::private(format!("Player '{}' not found!", alias)),
                    );
                }
            }
        }
        None => server
            .clients
            .notify(entity, ServerMsg::private(String::from(action.help_string))),
    }
}
