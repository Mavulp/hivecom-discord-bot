use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::ErrorKind::NotFound as FileNotFound;

use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::model::prelude::*;
use serenity::prelude::*;

use rawr::auth::AnonymousAuthenticator;
use rawr::client::RedditClient;
use rawr::options::ListingOptions;
use rawr::traits::{Content, Stickable};

use circular_queue::CircularQueue;

use failure::format_err;
use log::error;

use crate::check_msg;
use crate::ChannelStore;
use crate::Reddit;
use crate::Result;

#[command]
#[description("Gets an unseen URL from the last 500 on a subreddit of choice.")]
#[usage("SUBREDDIT <comments> <AGE>")]
#[only_in(guilds)]
fn reddit(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let subreddit = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide an argument"));

            return Ok(());
        }
    };

    let channel = if let Channel::Guild(channel) = msg.channel_id.to_channel(&ctx).unwrap() {
        channel
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Groups and DMs not supported"),
        );

        return Ok(());
    };
    let channel = channel.read();

    let result = {
        let mut data = ctx.data.write();
        let reddit = data
            .get_mut::<Reddit>()
            .expect("RedditSearch is in ShareMap.");

        if let Ok(arg) = args.single::<String>() {
            if arg != "comments" {
                check_msg(msg.channel_id.say(&ctx.http, "Unknown argument"));
                return Ok(());
            }
            let count = args.single::<usize>().unwrap_or(1);
            match reddit.get_comments(subreddit.clone(), msg.channel_id, count - 1) {
                Some((sub, id)) => Ok(format!("https://reddit.com/r/{}/comments/{}/", sub, id)),
                None => Err(format_err!(
                    "No post for that subreddit was sent previously"
                )),
            }
        } else {
            reddit.get_post(subreddit, msg.channel_id, channel.nsfw)
        }
    };

    match result {
        Ok(url) => check_msg(msg.channel_id.say(&ctx.http, &url)),
        //TODO Err msg
        Err(e) => check_msg(msg.channel_id.say(&ctx.http, &e.to_string())),
    }

    Ok(())
}

pub struct RedditSearch {
    client: RedditClient,
    known: HashMap<(ChannelId, String), CircularQueue<(String, String)>>,
}

impl RedditSearch {
    pub fn new() -> Self {
        RedditSearch {
            client: RedditClient::new("discord-bot", AnonymousAuthenticator::new()),
            known: HashMap::new(),
        }
    }

    pub fn get_comments(
        &mut self,
        sub: String,
        channel: ChannelId,
        count: usize,
    ) -> Option<(String, String)> {
        let known = self
            .known
            .entry((channel, sub))
            .or_insert(CircularQueue::with_capacity(500));

        known.iter().nth(count).map(|s| s.clone())
    }

    pub fn get_post(&mut self, sub: String, channel: ChannelId, nsfw: bool) -> Result<String> {
        let subreddit = self.client.subreddit(&sub);

        let hot = subreddit.hot(ListingOptions::default())?;

        let known = self
            .known
            .entry((channel, sub))
            .or_insert(CircularQueue::with_capacity(500));

        for post in hot.take(1000) {
            if post.stickied() || (!nsfw && post.nsfw()) {
                continue;
            }

            if let Some(url) = post.link_url() {
                let id = post.name()[3..].to_string();
                if let Some(_) = known.iter().find(|&(_, k)| k == &id) {
                    continue;
                }
                known.push((post.subreddit().name, id));

                return Ok(url);
            }
        }
        if nsfw {
            Err(format_err!("Found no URLs in most recent 1000 posts"))
        } else {
            Err(format_err!(
                "Found no SFW URLs in the most recent 1000 posts"
            ))
        }
    }
}

#[command("tempchan")]
#[description("Creates a temporary channel that will get removed once empty.")]
#[usage("<name>")]
#[only_in(guilds)]
fn temporary_channel(ctx: &mut Context, msg: &Message) -> CommandResult {
    let guild = match msg.guild(&ctx.cache) {
        Some(guild) => guild,
        None => {
            check_msg(msg.reply(&ctx, "Groups and DMs are not supported"));

            return Ok(());
        }
    };

    if let None = guild.read().voice_states.get(&msg.author.id) {
        check_msg(msg.reply(
            &ctx,
            "You must be in a voice channel to create a temporary channel",
        ));

        return Ok(());
    };

    let name = msg
        .content
        .find(' ')
        .map(|start| &msg.content[start + 1..])
        .unwrap_or("Temporary Channel");

    let channel = match guild
        .read()
        .create_channel(&ctx, |c| c.name(&name).kind(ChannelType::Voice))
    {
        Ok(c) => c,
        Err(e) => {
            check_msg(msg.reply(&ctx, &format!("Failed to create channel: {}", e)));
            return Ok(());
        }
    };

    let result = {
        let mut data = ctx.data.write();
        let chan_store = data
            .get_mut::<ChannelStore>()
            .expect("ChannelStore is in ShareMap.");
        chan_store.add_chan(&channel.id)
    };

    if let Err(e) = result {
        error!("Failed to save temp channel: {}", e);
        check_msg(msg.reply(&ctx, "Failed to store temporary channel"));
        ctx.http.delete_channel(channel.id.0)?;
        return Ok(());
    }

    if let Err(e) = guild
        .read()
        .move_member(&ctx.http, msg.author.id, channel.id)
    {
        error!("Failed to move user: {}", e);
        check_msg(msg.reply(&ctx, "Failed to move user"));
        ctx.http.delete_channel(channel.id.0)?;
        return Ok(());
    }

    Ok(())
}

pub fn check_temp_chans(ctx: &Context, guild_id: &GuildId) {
    let guild = guild_id
        .to_guild_cached(&ctx.cache)
        .expect("Guild is cached");

    let guild = guild.read();
    let mut data = ctx.data.write();
    let chan_store = data
        .get_mut::<ChannelStore>()
        .expect("ChannelStore is in ShareMap.");

    if let Ok(channels) = guild.channels(&ctx.http) {
        for (channel_id, _) in channels {
            if chan_store.contains(&channel_id) {
                if guild
                    .voice_states
                    .iter()
                    .filter(|(_, vs)| vs.channel_id.map(|id| channel_id == id) == Some(true))
                    .count()
                    < 1
                {
                    if let Err(e) = ctx.http.delete_channel(channel_id.0) {
                        error!("Failed to delete temporary channel: {}", e);
                    } else {
                        if let Err(e) = chan_store.remove_chan(&channel_id) {
                            error!("{}", e);
                        }
                    }
                }
            }
        }
    }
}

pub struct TempChannelStore {
    temp_chans: Vec<u64>,
}

impl TempChannelStore {
    pub fn new() -> Self {
        let result = OpenOptions::new().read(true).open("db/channels.bin");

        match result {
            Ok(f) => {
                let chans: Vec<u64> = match bincode::deserialize_from(f) {
                    Ok(cs) => cs,
                    Err(e) => {
                        error!("Failed to deserialize channels: {}", e);
                        Vec::new()
                    }
                };

                TempChannelStore { temp_chans: chans }
            }
            Err(e) => {
                // FIXME What
                if FileNotFound != e.kind() {
                    error!("Failed to open channel file: {}", e);
                }

                TempChannelStore {
                    temp_chans: Vec::new(),
                }
            }
        }
    }

    pub fn add_chan(&mut self, id: &ChannelId) -> Result<()> {
        self.temp_chans.push(id.0);
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open("db/channels.bin")?;

        bincode::serialize_into(file, &self.temp_chans)?;

        Ok(())
    }
    pub fn remove_chan(&mut self, id: &ChannelId) -> Result<()> {
        let idx = self
            .temp_chans
            .iter()
            .enumerate()
            .find(|&(_, c)| c == &id.0)
            .map(|(i, _)| i)
            .ok_or(format_err!("ChannelId not found"))?;
        self.temp_chans.remove(idx);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open("db/channels.bin")?;

        bincode::serialize_into(file, &self.temp_chans)?;
        Ok(())
    }

    pub fn contains(&self, id: &ChannelId) -> bool {
        self.temp_chans.contains(&id.0)
    }
}
