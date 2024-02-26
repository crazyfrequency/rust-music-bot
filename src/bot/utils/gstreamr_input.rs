use std::io::{Read, Result as IoResult};
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use actix_web::web::Buf;
use gstreamer::{buffer, prelude::*, Buffer, ClockTime, SeekFlags};

use gstreamer::Element;
use songbird::input::{AudioStream, Input, LiveInput};
use symphonia::core::io::{MediaSource, ReadOnlySource};

pub struct GstreamerInput {
    pub src: Element,
    pub sink: gstreamer_app::AppSink,
    pub buffer: Option<Vec<u8>>
}

impl GstreamerInput {
    pub fn new(uri: &str) -> Option<Self> {
        let pipeline = gstreamer::parse_launch(format!("uridecodebin uri=\"{}\" ! audioconvert ! flacenc ! appsink name=sink", uri).as_str()).ok()?;
        let pipeline_copy = pipeline.clone();
        let sink = pipeline.clone().dynamic_cast::<gstreamer::Pipeline>().ok()?.by_name("sink")?.dynamic_cast::<gstreamer_app::AppSink>().unwrap();
        pipeline.set_state(gstreamer::State::Playing);
        let bus = pipeline.bus().expect("Pipeline without bus. Shouldn't happen!");
        
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(20));
            pipeline_copy.seek_simple(SeekFlags::FLUSH, ClockTime::from_seconds(520));
            for msg in bus.iter_timed(ClockTime::NONE) {
                use gstreamer::MessageView;

                match msg.view() {
                    MessageView::ClockLost(_) => println!("cl lost"),
                    MessageView::Warning(err) => {
                        eprintln!("Error from {:?}: {}", err.src().map(|s| s.path_string()), err.error());
                        eprintln!("Debugging information: {:?}", err.debug());
                        pipeline_copy.set_state(gstreamer::State::Null).unwrap();
                        // Задержка перед переподключением
                        thread::sleep(std::time::Duration::from_secs(5));
                        pipeline_copy.set_state(gstreamer::State::Playing).unwrap();
                        break;
                    },
                    MessageView::Error(err) => {
                        eprintln!("Error from {:?}: {}", err.src().map(|s| s.path_string()), err.error());
                        eprintln!("Debugging information: {:?}", err.debug());
                        pipeline_copy.set_state(gstreamer::State::Null).unwrap();
                        // Задержка перед переподключением
                        thread::sleep(std::time::Duration::from_secs(5));
                        pipeline_copy.set_state(gstreamer::State::Playing).unwrap();
                        break;
                    },
                    MessageView::Eos(..) => break, // end-of-stream, мы закончили
                    _ => (),
                }
            }
        });
        
        Some(Self {
            src: pipeline,
            sink,
            buffer: None
        })
    }
}

impl Read for GstreamerInput {
    fn read(&mut self, buffer: &mut [u8]) -> IoResult<usize> {
        if self.sink.is_eos() {
            println!("hm");
            return Ok(0);
        }
        println!("{:?}", self.sink.state(ClockTime::from_seconds(1)));
        match self.buffer.as_ref() {
            Some(buffer_ref) => {
                let mut reader = buffer_ref.reader();
                let res = reader.read(buffer);
                let mut vector = Vec::new();
                let _ = reader.read_to_end(&mut vector);
                self.buffer = Some(vector);
                match res {
                    Ok(0) => match self.sink.try_pull_sample(Some(ClockTime::from_seconds(5))) {
                        Some(sample) => {
                            // println!("1");
                            self.buffer = Some(sample.buffer().take().unwrap().map_readable().unwrap().to_vec());
                            println!("{} {}", self.buffer.as_ref().unwrap().len(), buffer.len());
                            let mut reader = self.buffer.as_ref().unwrap().reader();
                            let res = reader.read(buffer);
                            let mut vector = Vec::new();
                            let _ = reader.read_to_end(&mut vector);
                            self.buffer = Some(vector);
                            println!("{:?} {:?}", self.buffer, res);
                            res
                        },
                        _ => Ok(0)
                    },
                    Ok(size) => {
                        println!("{}", size);
                        Ok(size)
                    },
                    Err(_) => Ok(0)
                }
            },
            _ => match self.sink.try_pull_sample(Some(ClockTime::from_seconds(5))) {
                Some(sample) => {
                    self.buffer = Some(sample.buffer().take().unwrap().map_readable().unwrap().to_vec());
                    println!("{} {}", self.buffer.as_ref().unwrap().len(), buffer.len());
                    let mut reader = self.buffer.as_ref().unwrap().reader();
                    let res = reader.read(buffer);
                    let mut vector = Vec::new();
                    let _ = reader.read_to_end(&mut vector);
                    self.buffer = Some(vector);
                    println!("{:?} {:?}", self.buffer, res);
                    res
                },
                _ => Ok(0)
            }
        }
        
    }
}

impl From<GstreamerInput> for Input {
    fn from(val: GstreamerInput) -> Self {
        let audio_stream = AudioStream {
            input: Box::new(ReadOnlySource::new(val)) as Box<dyn MediaSource>,
            hint: None,
        };
        Input::Live(LiveInput::Raw(audio_stream), None)
    }
}