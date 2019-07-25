use rayon::prelude::*;
use image::{ RgbImage };

//use std::io::Read;
use std::io::BufReader;
//use byteorder::{LittleEndian, ReadBytesExt};
//use rand::{ Rng};


use arcstar::detector::*;
use arcstar::sae_types::*;
use eventcam_tracker::tracker::FeatureTracker;

use eventcam_converter::conversion;
use std::fs::create_dir_all;
use std::path::Path;

const MAX_FRAMES: u32 = 3599;


pub const RED_PIXEL: [u8; 3] = [255u8, 0, 0];
pub const GREEN_PIXEL: [u8; 3] = [0, 255u8, 0];
pub const YELLOW_PIXEL: [u8; 3] = [255u8, 255u8, 0];
pub const BLUE_PIXEL: [u8; 3] = [0,0,  255u8];



pub fn render_matches(nrows: u32, ncols: u32, matches: &Vec<(SaeEvent,SaeEvent)> ) -> RgbImage {
  let mut out_img =   RgbImage::new(ncols, nrows);

  for (new_evt, old_evt) in matches {
    out_img.put_pixel(new_evt.col as u32, new_evt.row as u32, image::Rgb(RED_PIXEL));
    out_img.put_pixel(old_evt.col as u32, old_evt.row as u32, image::Rgb(GREEN_PIXEL));

  }

  out_img
}

/// render corners into an image result
pub fn render_corners(nrows: u32, ncols: u32, events: &Vec<SaeEvent> ) -> RgbImage {
  let mut out_img =   RgbImage::new(ncols, nrows);

  for evt in events {
    let px = match evt.polarity {
      1 => image::Rgb(YELLOW_PIXEL),
      0 => image::Rgb(GREEN_PIXEL),
      _ => unreachable!()
    };

    out_img.put_pixel(evt.col as u32, evt.row as u32, px);
  }

  out_img
}

pub fn render_sae(nrows: u32, ncols: u32, sae_rise: &SaeMatrix, sae_fall: &SaeMatrix,  time_horizon: SaeTime ) -> RgbImage {
  let mut out_img =   RgbImage::new(ncols, nrows);

  for row in 0..nrows {
    for col in 0..ncols {
      let sae_rise_val: SaeTime = sae_rise[(row as usize, col as usize)] ;
      let sae_fall_val: SaeTime = sae_fall[(row as usize, col as usize)] ;
      let sae_val = sae_rise_val.max(sae_fall_val);
      if 0 != sae_val && sae_val > time_horizon {
        let mut blue_val = 0;
        let total_val = (sae_fall_val + sae_rise_val) as f32;
        let mut red_val: u8 = (255.0*(sae_rise_val as f32)/total_val) as u8;
        let mut green_val: u8 = (255.0*(sae_fall_val as f32)/total_val) as u8;
        if red_val < 255 && green_val < 255 {
          blue_val = 255;
          red_val = 0;
          green_val = 0;
        }
        let px_data: [u8; 3] = [red_val, green_val, blue_val];
        let px =  image::Rgb(px_data);

        out_img.put_pixel(col as u32, row as u32, px);
      }
    }
  }

  out_img
}

fn process_events(tracker: &mut FeatureTracker, sae_rise: &mut SaeMatrix,  sae_fall: &mut SaeMatrix, event_list: &Vec<SaeEvent>) -> Vec<(SaeEvent, SaeEvent)>  {

  //update the SAE first //TODO handle one event at at a time?
  for evt in event_list {
    let row = evt.row as usize;
    let col = evt.col  as usize;
    match evt.polarity {
      1 => sae_rise[(row, col)] = evt.timestamp,
      _ => sae_fall[(row, col)] = evt.timestamp,
    };
  }

  // we can check corners in parallel because SAE has already been updated,
  // and can now be considered read-only
  let corners:Vec<SaeEvent> = event_list.par_iter().filter_map(|evt| {
    detect_and_compute_one(&sae_rise, &sae_fall, evt)
  }).collect();

  let matches:Vec<(SaeEvent, SaeEvent)> = corners.iter().filter_map(|new_evt: &SaeEvent| {
    let matched_parent = tracker.add_and_match_feature(&new_evt);
    match matched_parent.is_some() {
      true => Some( (new_evt.clone(), matched_parent.unwrap()) ),
      false => None
    }
  }).collect();

  println!("events: {} corners: {} matches: {}", event_list.len(), corners.len(), matches.len() );
  matches
}


/// process an event file into a set of corners
pub fn process_event_file(src_path: &str, img_w: u32, img_h: u32, render_out: bool) {

  let event_file_res = std::fs::File::open(src_path);
  if event_file_res.is_err() {
    println!("No event file...skipping");
    return;
  }
  let event_file = event_file_res.unwrap();
  let mut buf_reader = BufReader::new(event_file);

  // The Surface of Active Events (timestamps for last event at each pixel point)
  let mut  sae_rise = SaeMatrix::zeros(
    img_h as usize, // rows
    img_w as usize // cols
  );

  let mut  sae_fall = SaeMatrix::zeros(
    img_h as usize, // rows
    img_w as usize // cols
  );

  let mut tracker = Box::new(FeatureTracker::new());

  //ensure that output directory exists
  create_dir_all(Path::new("./out/")).expect("Couldn't create output dir");


  let mut chunk_count = 0;
  loop {
    chunk_count += 1;

    let timebase:f64 = 0.003811000; //from slider events.txt file -- //TODO standardize
    let timescale:f64 = 1E-6; //one microsecond per SaeTime tick
    let event_list = conversion::read_next_chunk_sae_events(&mut buf_reader, timebase, timescale);

    if event_list.len() > 0 {
      let matches:Vec<(SaeEvent,SaeEvent)> = process_events(&mut tracker, &mut sae_rise, &mut sae_fall, &event_list);

      //TODO fix rendering
      if render_out {
        let lead_events = matches.iter().map(|(new, _old)| new.clone()).collect();
        let out_img = render_corners(img_h, img_w, &lead_events);
        let out_path = format!("./out/sae_{:04}_evts.png", chunk_count);
        out_img.save(out_path).expect("Couldn't save");
      }

//      if render_out {
//        let max_time_delta = 5*frame_time_delta;
//        let horizon = timestamp.max(max_time_delta) - max_time_delta;
//        let out_img = render_sae(img_h, img_w, &sae_rise, &sae_fall, horizon);
//        let out_path = format!("./out/saesurf_{:04}.png", chunk_count);
//        out_img.save(out_path).expect("Couldn't save");
//      }

//      if render_out {
//        let out_path = format!("./out/sae_{:04}_tracks.png", chunk_count);
//        let lead_time_horizon = timestamp.max(FORGETTING_TIME) - FORGETTING_TIME;
//        tracker.render_tracks_to_file(img_h, img_w,  lead_time_horizon, &out_path); //TODO check timestamp
//      }
      
    }
    else {
      break;
    }

    if chunk_count > MAX_FRAMES {
      break;
    }
  }


}



fn main() {
  //TODO get image / SAE dimensions from configuration?
  let img_w = 320;
  let img_h = 320;

  process_event_file("./data/events.dat", img_w, img_h, true);

}




