use std::{io::Write, sync::Arc, process::{Command, Stdio}};

use actix_web::{get, web::{self, get, Path}, Responder};
use serde_json::Number;
use serenity::all::GuildId;
use songbird::Songbird;

use crate::bot::utils::player::PlayerDataType;

#[get("/{id}/{value}")]
async fn test(params: Path<(u64, f64)>, data: web::Data<Arc<Songbird>>) -> impl Responder {
  // data is a borrowed version of the state
  let guild_data = data.get(GuildId::from(params.0)).unwrap();
  let mut handler = guild_data.lock().await;
  let child = Command::new("ffmpeg").args([
      "-reconnect", "1", "-reconnect_streamed", "1",
      "-reconnect_delay_max", "5", "-err_detect", "ignore_err",
      "-vn", "-sn", "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:106.0) Gecko/20100101 Firefox/106.0",
      "-i", "https://cdn.freesound.org/previews/45/45515_111413-lq.mp3",
      "-f", "wav", "pipe:1"
    ]).stdin(Stdio::null())
    .stderr(Stdio::null())
    .stdout(Stdio::piped()).spawn().unwrap();
  let data = songbird::input::Input::from(songbird::input::ChildContainer::from(child));
  let handle = handler.play_input(data);

  format!("ok")

  // match data {
  //     Some(data) => {
  //       let data = data.clone();
  //       let mut settings = data.settings.write().await;
  //       let mut ffmpeg = data.ffmpeg.write().await;
  //       settings.volume = params.1 * 0.2;
  //       let stdin = ffmpeg.as_mut();
  //       match stdin {
  //           Some(stdin) => {
  //               let _ = stdin.write(format!("^Cvolume -1 volume {}\n", settings.volume).as_bytes());
  //               println!("1");
  //           }, None => {}
  //       }
  //       format!("{:?}", data)
  //     },
  //     None => {"null".to_string()}
  // }
}