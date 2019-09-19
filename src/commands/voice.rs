use crate::check_msg;
use crate::VoiceManager;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use serenity::voice;

use log::error;

#[command]
#[only_in(guilds)]
#[description("Joins the voice channel the sender is in.")]
fn join(ctx: &mut Context, msg: &Message) -> CommandResult {
    let user_id = msg.author.id;
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            check_msg(msg.reply(&ctx, "Groups and DMs are not supported"));

            return Ok(());
        }
    };
    let guild = guild.read();

    let channel_id = match guild
        .voice_states
        .get(&user_id)
        .and_then(|state| state.channel_id)
    {
        Some(id) => id,
        None => {
            check_msg(msg.reply(&ctx, "You are not in a voice channel"));

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("VoiceManager is in ShareMap.");
    let mut manager = manager_lock.lock();

    if let None = manager.join(guild.id, channel_id) {
        check_msg(msg.channel_id.say(&ctx.http, "Error joining the channel"));
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
#[description("Leaves the voice channel.")]
fn leave(ctx: &mut Context, msg: &Message) -> CommandResult {
    let guild_id = match ctx.cache.read().guild_channel(msg.channel_id) {
        Some(channel) => channel.read().guild_id,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Groups and DMs not supported"),
            );

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("VoiceManager is in ShareMap.");
    let mut manager = manager_lock.lock();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        manager.remove(guild_id);
    } else {
        check_msg(msg.reply(&ctx, "Not in a voice channel"));
    }

    Ok(())
}

#[command]
#[description("Plays audio in the current voice channel.")]
#[usage("URI")]
#[only_in(guilds)]
fn play(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let uri = match args.single::<String>() {
        Ok(uri) => uri,
        Err(_) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Must provide a URI to a video or audio"),
            );

            return Ok(());
        }
    };

    let guild_id = match ctx.cache.read().guild_channel(msg.channel_id) {
        Some(channel) => channel.read().guild_id,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Groups and DMs not supported"),
            );

            return Ok(());
        }
    };

    let manager_lock = ctx
        .data
        .read()
        .get::<VoiceManager>()
        .cloned()
        .expect("VoiceManager is in ShareMap");
    let mut manager = manager_lock.lock();

    if let Some(handler) = manager.get_mut(guild_id) {
        let source = match voice::ytdl(&uri) {
            Ok(source) => source,
            Err(why) => {
                error!("Err starting source: {:?}", why);

                check_msg(msg.channel_id.say(&ctx.http, "Failed to play URI"));

                return Ok(());
            }
        };

        handler.play(source);

        check_msg(msg.channel_id.say(&ctx.http, "Playing song"));
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in"),
        );
    }

    Ok(())
}
