use failure::bail;
use rand::{thread_rng, Rng};
use reqwest::{header, Client};
use url::form_urlencoded;

use crate::check_msg;
use crate::Result;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[aliases(r34)]
#[description("Searches rule34.xxx for images and returns one at random.")]
#[usage("TAGS")]
#[only_in(guilds)]
fn rule34(ctx: &mut Context, msg: &Message) -> CommandResult {
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

    if !channel.nsfw {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "This command only works in NSFW channels"),
        );

        return Ok(());
    }

    let content = &msg.content;
    let tags = content
        .split(' ')
        .skip(1)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    // TODO Terrible
    let tags = tags
        .join(" ")
        .replace(", ", ",")
        .replace(' ', "_")
        .replace(',', " ");

    if tags.is_empty() {
        check_msg(msg.channel_id.say(&ctx.http, "Specify at least one tag"));
        return Ok(());
    }

    let response = match get_amount(&tags) {
        Ok(amount) => {
            if amount < 1 {
                check_msg(msg.channel_id.say(&ctx.http, "No Results"));
                return Ok(());
            }

            let mut rng = thread_rng();
            let rand: u64 = rng.gen_range(0, amount);

            match get_url(&tags, rand) {
                Ok(v) => v,
                Err(e) => e.to_string(),
            }
        }
        Err(e) => e.to_string(),
    };

    check_msg(msg.channel_id.say(&ctx.http, &response));

    Ok(())
}

fn get_amount(tags: &str) -> Result<u64> {
    use std::str::FromStr;

    let url: String = form_urlencoded::Serializer::new(String::from(
        "https://rule34.xxx/index.php?page=dapi&s=post&q=index",
    ))
    .append_pair("limit", "0")
    .append_pair("pid", "0")
    .append_pair("tags", tags)
    .finish();

    Ok(u64::from_str(&get_attribute(&url, "count")?)?)
}

fn get_url(tags: &str, pid: u64) -> Result<String> {
    let url: String = form_urlencoded::Serializer::new(String::from(
        "https://rule34.xxx/index.php?page=dapi&s=post&q=index",
    ))
    .append_pair("limit", "1")
    .append_pair("pid", &pid.to_string())
    .append_pair("tags", tags)
    .finish();

    get_attribute(&url, "file_url")
}

fn get_attribute(url: &str, name: &str) -> Result<String> {
    use xml::reader::{EventReader, XmlEvent};

    let response = Client::new()
        .post(url)
        .header(header::CONNECTION, "close")
        .send()?;

    let parser = EventReader::new(response);

    for ev in parser {
        match ev {
            Ok(XmlEvent::StartElement { attributes, .. }) => {
                for attr in attributes {
                    if attr.name.local_name == name {
                        return Ok(attr.value.clone());
                    } else if attr.name.local_name == "reason" {
                        bail!("No results: {}", attr.value);
                    }
                }
            }
            _ => (),
        }
    }
    bail!("API deserialization failed");
}
