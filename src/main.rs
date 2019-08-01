
#[macro_use]
extern crate clap;

use std::io::BufReader;
use arcstar::sae_types::*;
use eventcam_tracker::tracker::FeatureTracker;

use eventcam_converter::conversion;
use std::fs::create_dir_all;
use std::path::Path;


/// process an event file into a set of corners
/// # Arguments
///
/// * `src_path` - Path to the input data file, eg `./data/events.dat`
/// * `img_w` - Width of the camera input, in pixels
/// * `img_h` - Height of the camera input, in pixels
/// * `timebase` - A start time for the event file processing (seconds from some absolute zero)
/// * `timescale` - Seconds per SaeTime tick
/// * `max_events` - A limit to the number of events to process, or 0 if no limit
/// * `render_events` - Should we render input events to an image file?
/// * `render_sae` - Should we render the Surface of Active Events to an image file?
/// * `render_corners` - Should we render detected corners (features) to an image file?
/// * `render_tracks` - Should we render feature tracks to an image file?
///
pub fn process_event_file(src_path: &Path,
                          img_w: u32, img_h: u32,
                          timebase: f64,
                          timescale: f64,
                          max_events: usize,
                          render_events: bool,
                          render_sae: bool,
                          render_corners: bool,
                          render_tracks: bool) {

  let event_file_res = std::fs::File::open(src_path);
  if event_file_res.is_err() {
    println!("No event file...skipping");
    return;
  }
  let event_file = event_file_res.unwrap();
  let mut event_reader = BufReader::new(event_file);

  let time_window = (0.1 / timescale) as SaeTime; //0.1 second
  //TODO get reference time filter threshold from command line option?
  let ref_time_filter = (50E-3 / timescale) as SaeTime; //50ms
  let mut tracker = Box::new(FeatureTracker::new(img_w, img_h, time_window, ref_time_filter));

  //ensure that output directory exists
  create_dir_all(Path::new("./out/")).expect("Couldn't create output dir");

  let mut chunk_count = 0;
  let mut event_count = 0;
  loop {
    chunk_count += 1;

    let event_list_opt = conversion::read_next_chunk_sae_events(&mut event_reader, timebase, timescale);
    if event_list_opt.is_none() {
      break;
    }
    let event_list:Vec<SaeEvent> = event_list_opt.unwrap();

    if event_list.len() > 0 {
      event_count += event_list.len();
      let corners:Vec<SaeEvent> = tracker.process_events(&event_list);
      println!("chunk: {} events: {} corners: {} ", chunk_count, event_list.len(), corners.len() );

      //TODO configure horizon based on command line options
      let timestamp = event_list.first().unwrap().timestamp;
      let horizon = timestamp.max(time_window) - time_window;

      if render_events {
        let out_path = format!("./out/sae_{:04}_events.png", chunk_count);
        tracker.render_events_to_file( &event_list, &FeatureTracker::RED_PIXEL, &FeatureTracker::BLUE_PIXEL, &out_path );
      }
      if render_corners {
        let out_path = format!("./out/sae_{:04}_corners.png", chunk_count);
        tracker.render_corners_to_file( &corners, &FeatureTracker::YELLOW_PIXEL, &FeatureTracker::GREEN_PIXEL, &out_path );
      }
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
      println!("no more events after {} chunks {} events", chunk_count, event_count);
      break;
    }

    if max_events > 0 && event_count > max_events {
      println!("Stopping on limit of {} events",event_count);
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
        (@arg TIMEBASE: --timebase +takes_value  "Base time, default is 0.0 seconds")
        (@arg TIMESCALE: --timescale +takes_value "Seconds per clock tick, default is 1E-6")
        (@arg MAX_EVENTS: --max_events +takes_value "Maximum number of events to process")
        (@arg RNDR_EVENTS: --rend_events  "Render events as they occur")
        (@arg RNDR_SAE: --rend_sae  "Render the Surface of Active Events (SAE)")
        (@arg RNDR_CORNERS: --rend_corners  "Render corners as they're detected")
        (@arg RNDR_TRACKS: --rend_tracks   "Render feature matches as tracks:")
    ).get_matches();

  //println!("clap matches: {:?}", matches);

  let infile = matches.value_of("INPUT").unwrap_or("./data/events.dat");
  let img_w = matches.value_of("WIDTH").unwrap_or("240").parse::<u32>().unwrap();
  let img_h = matches.value_of("HEIGHT").unwrap_or("180").parse::<u32>().unwrap();
  let timebase = matches.value_of("TIMEBASE").unwrap_or("0.0").parse::<f64>().unwrap();
  let timescale = matches.value_of("TIMESCALE").unwrap_or("1E-6").parse::<f64>().unwrap();
  let max_events = matches.value_of("MAX_EVENTS").unwrap_or("0").parse::<usize>().unwrap();
  let render_events = matches.is_present("RNDR_EVENTS");
  let render_sae = matches.is_present("RNDR_SAE");
  let render_corners = matches.is_present("RNDR_CORNERS");
  let render_tracks = matches.is_present("RNDR_TRACKS");


  let in_path = Path::new(infile);
  if !in_path.exists() {
    eprintln!("Input file doesn't exist: {}", infile);
  }
  else {
    println!("Reading from {}", infile);
  }

  process_event_file(&in_path, img_w, img_h, timebase, timescale, max_events, render_events, render_sae, render_corners, render_tracks);

}




