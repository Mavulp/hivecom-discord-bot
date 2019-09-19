use crate::check_msg;
use crate::ShardManagerContainer;
use serenity::framework::standard::{macros::command, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
#[owners_only]
fn quit(ctx: &mut Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read();

    if let Some(manager) = data.get::<ShardManagerContainer>() {
        let _ = msg.reply(&ctx, "Shutting down!").unwrap();
        manager.lock().shutdown_all();
    } else {
        check_msg(msg.reply(&ctx, "There was a problem getting the shard manager"));

        return Ok(());
    }

    Ok(())
}
