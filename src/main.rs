
#[macro_use]
extern crate clap;

use std::io::BufReader;
use arcstar::sae_types::*;
use eventcam_tracker::tracker::FeatureTracker;

use eventcam_converter::conversion;
use std::fs::create_dir_all;
use std::path::Path;

//TODO allow timebase, scale, window to be defined from command line

const TMAX_FORGETTING_TIME: SaeTime = (0.1 / 1E-6) as SaeTime;
const TIMEBASE:f64 = 0.001; //0.003811000; //from slider events.txt file -- //TODO standardize
const TIME_SCALE:f64 = 1E-6; //one microsecond per SaeTime tick
const FRAME_TIME_DELTA:SaeTime = TMAX_FORGETTING_TIME; // 10 ms / 0.01 sec given the timescale above
const MAX_TIME_DELTA:SaeTime = FRAME_TIME_DELTA;


/// process an event file into a set of corners
pub fn process_event_file(src_path: &Path, img_w: u32, img_h: u32,  render_sae: bool, render_tracks: bool) {

  let event_file_res = std::fs::File::open(src_path);
  if event_file_res.is_err() {
    println!("No event file...skipping");
    return;
  }
  let event_file = event_file_res.unwrap();
  let mut event_reader = BufReader::new(event_file);
  let mut tracker = Box::new(FeatureTracker::new(img_w, img_h, TMAX_FORGETTING_TIME));

  //ensure that output directory exists
  create_dir_all(Path::new("./out/")).expect("Couldn't create output dir");

  let mut chunk_count = 0;
  loop {
    chunk_count += 1;

    let event_list_opt = conversion::read_next_chunk_sae_events(&mut event_reader, TIMEBASE, TIME_SCALE);
    if event_list_opt.is_none() {
      break;
    }
    let event_list = event_list_opt.unwrap();

    if event_list.len() > 0 {
      let corners:Vec<SaeEvent> = tracker.process_events(&event_list);
      println!("chunk: {} events: {} corners: {} ", chunk_count, event_list.len(), corners.len() );

      //TODO configure horizon based on command line options
      let timestamp = event_list.first().unwrap().timestamp;
      let horizon = timestamp.max(MAX_TIME_DELTA) - MAX_TIME_DELTA;

      if render_sae {
        let out_path = format!("./out/saesurf_{:04}.png", chunk_count);
        tracker.render_sae_frame_to_file(horizon, &out_path);
      }

      if render_tracks {
        let out_path= format!("./out/sae_{:04}_tracks.png", chunk_count);
        tracker.render_tracks_to_file(horizon, &out_path);
      }
    }
    else {
      println!("no more events after {} chunks", chunk_count);
      break;
    }
  }


}



fn main() {

  let matches = clap_app!(blink_cam =>
        (version: "0.1.0")
        (author: "Todd Stellanova")
        (about: "Process event camera pixel change event stream into tracked features.")
        (@arg INPUT: -i --input +takes_value  "Sets the input file to use: default is ./data/events.dat")
        (@arg WIDTH: --width +takes_value  "Sets the input width to use: default is 240 pixels")
        (@arg HEIGHT: --height +takes_value  "Sets the input height to use: default is 180 pixels")
        (@arg RNDR_SAE: --rend_sae  "Render the Surface of Active Events (SAE)")
        (@arg RNDR_TRACKS: --rend_tracks   "Render feature matches as tracks:")
    ).get_matches();

  //println!("clap matches: {:?}", matches);

  let infile = matches.value_of("INPUT").unwrap_or("./data/events.dat");
  let img_w = matches.value_of("WIDTH").unwrap_or("240").parse::<u32>().unwrap();
  let img_h = matches.value_of("HEIGHT").unwrap_or("180").parse::<u32>().unwrap();

  let render_sae = matches.is_present("RNDR_SAE");
  let render_tracks = matches.is_present("RNDR_TRACKS");

  println!("render_tracks: {}", render_tracks);

  let in_path = Path::new(infile);
  if !in_path.exists() {
    eprintln!("Input file doesn't exist: {}", infile);
  }
  else {
    println!("Reading from {}", infile);
  }

  process_event_file(&in_path, img_w, img_h, render_sae, render_tracks);

}




