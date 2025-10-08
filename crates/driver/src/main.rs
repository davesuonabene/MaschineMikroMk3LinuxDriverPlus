mod self_test;
mod settings;

use crate::self_test::self_test;
use crate::settings::{Settings, ButtonMode}; 
use clap::Parser;
use config::Config;
use hidapi::{HidDevice, HidResult}; 
use maschine_library::controls::{Buttons, PadEventType};
use maschine_library::lights::{Brightness, Lights, PadColors};
use maschine_library::screen::Screen;
use midir::os::unix::VirtualOutput;
use midir::{MidiOutput, MidiOutputConnection};
use midly::{MidiMessage, live::LiveEvent};

use rosc::{OscMessage, OscPacket, OscType};
use std::net::{UdpSocket, ToSocketAddrs};
use std::error::Error as StdError; 
use std::collections::HashMap; 

#[derive(Parser, Debug)]
#[clap(
    name = "Maschine Mikro MK3 Userspace MIDI driver",
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
)]
struct Args {
    #[clap(short, long, help = "Config file (see example_config.toml)")]
    config: Option<String>,
}

fn main() -> Result<(), Box<dyn StdError>> {
    let args = Args::parse();

    let mut cfg = Config::builder();

    if let Some(config_fn) = args.config {
        cfg = cfg.add_source(config::File::with_name(config_fn.as_str()));
    }

    let cfg = cfg.build().expect("Can't create settings");
    let settings: Settings = cfg.try_deserialize().expect("Can't parse settings");

    settings.validate().unwrap();

    println!("Running with settings:");
    println!("{settings:?}");

    // --- OSC INITIALIZATION ---
    let osc_socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket for OSC");
    let osc_addr: std::net::SocketAddr = format!("{}:{}", settings.osc_ip, settings.osc_port)
        .to_socket_addrs()?
        .next().unwrap();
    println!("OSC output initialized to {}", osc_addr);
    // --- END OSC INITIALIZATION ---

    let output = MidiOutput::new(&settings.client_name).expect("Couldn't open MIDI output");
    let mut port = output
        .create_virtual(&settings.port_name)
        .expect("Couldn't create virtual port");

    let api = hidapi::HidApi::new()?;
    #[allow(non_snake_case)]
    let (VID, PID) = (0x17cc, 0x1700);
    let device = api.open(VID, PID)?;

    device.set_blocking_mode(false)?;

    let mut screen = Screen::new();
    let mut lights = Lights::new();

    self_test(&device, &mut screen, &mut lights)?;

    main_loop(
        &device, 
        &mut screen, 
        &mut lights, 
        &mut port, 
        &settings, 
        &osc_socket, 
        &osc_addr
    ).map_err(|e| Box::<dyn StdError>::from(e))?; 
    
    Ok(())
}

fn main_loop(
    device: &HidDevice,
    _screen: &mut Screen,
    lights: &mut Lights,
    port: &mut MidiOutputConnection,
    settings: &Settings,

    osc_socket: &UdpSocket,
    osc_addr: &std::net::SocketAddr,

) -> HidResult<()> {
    
    // Track the internal ON/OFF state for all toggle buttons
    let mut toggle_states: HashMap<Buttons, bool> = HashMap::new();
    // NEW: Track the last encoder value reported to suppress repeat printing of sticky values
    let mut last_encoder_val: u8 = 0; 
    
    let mut buf = [0u8; 64];
    loop {
        let size = device.read_timeout(&mut buf, 10)?;
        if size < 1 {
            continue;
        }

        let mut changed_lights = false;
        if buf[0] == 0x01 {
            // button mode
            for i in 0..6 {
                // bytes
                for j in 0..8 {
                    // bits
                    let idx = i * 8 + j;
                    let button: Option<Buttons> = num::FromPrimitive::from_usize(idx);
                    let button = match button {
                        Some(val) => val,
                        None => continue,
                    };
                    
                    let status = buf[i + 1] & (1 << j);
                    let is_pressed = status > 0;
                    
                    // Look up configuration
                    let button_name = format!("{:?}", button).to_string();
                    let config = settings.button_configs.get(&button_name);
                    let mode = config.map(|c| c.mode).unwrap_or_default();
                    
                    let current_light_state = lights.get_button(button) != Brightness::Off;
                    
                    let mut should_send_osc = false;
                    let mut osc_value: i32 = 0;
                    let mut target_light_brightness: Option<Brightness> = None;
                    
                    match mode {
                        ButtonMode::Trigger => {
                            // Trigger: Send 1 on press, 0 on release (only on state transition)
                            if is_pressed != current_light_state {
                                should_send_osc = true;
                                osc_value = if is_pressed { 1 } else { 0 };
                                target_light_brightness = Some(if is_pressed { 
                                    Brightness::Normal 
                                } else { 
                                    Brightness::Off 
                                });
                            }
                        }
                        ButtonMode::Toggle => {
                            // FIX: Only trigger the toggle state change if the button is pressed 
                            // AND the light is NOT currently Bright (to debounce the press).
                            if is_pressed && lights.get_button(button) != Brightness::Bright { 
                                
                                let current_toggle_state = *toggle_states.entry(button).or_insert(false);
                                let new_toggle_state = !current_toggle_state;
                                
                                toggle_states.insert(button, new_toggle_state);
                                should_send_osc = true;
                                // FIX: OSC value is correctly set here for both 1 (ON) and 0 (OFF)
                                osc_value = if new_toggle_state { 1 } else { 0 }; 
                                
                                // Set light state to Bright (fully ON cue)
                                target_light_brightness = Some(Brightness::Bright);
                            }
                            
                            // Handle light release feedback (Dim = ON, Released)
                            if !is_pressed && current_light_state {
                                // Check the current state of the toggle
                                if *toggle_states.get(&button).unwrap_or(&false) {
                                    // If toggle is ON, but button released, set dim light (visual feedback)
                                    target_light_brightness = Some(Brightness::Dim); 
                                } else {
                                    // If toggle is OFF, turn light off (this is the state after OSC 0 is sent)
                                    target_light_brightness = Some(Brightness::Off);
                                }
                            }
                        }
                    }
                    
                    // Handle OSC sending
                    if should_send_osc {
                        let address = format!("/maschine/{}", button_name.to_lowercase());
                        let msg_contents = OscMessage {
                            addr: address,
                            args: vec![OscType::Int(osc_value)], 
                        };
                        let packet = OscPacket::Message(msg_contents);

                        if let Ok(encoded_buf) = rosc::encoder::encode(&packet) {
                            if let Err(e) = osc_socket.send_to(&encoded_buf, osc_addr) {
                                eprintln!("Failed to send OSC message for {:?}: {}", button, e);
                            }
                        }
                    }
                    
                    // Handle light update
                    if let Some(b) = target_light_brightness {
                        if lights.button_has_light(button) {
                            lights.set_button(button, b);
                            changed_lights = true;
                        }
                    }

                    // Log press if it's a trigger
                    if mode == ButtonMode::Trigger && is_pressed {
                        println!("{:?}", button); 
                    }
                }
            }
            
            // --- FIX: Implement state tracking for encoder value ---
            let encoder_val = buf[7];
            
            // Only print if the value is non-zero (movement) OR if the value has changed back to zero 
            // from a previous non-zero value (solving the "never goes to zero" observation).
            if encoder_val != 0 || last_encoder_val != encoder_val {
                println!("Encoder: {}", encoder_val);
            }
            // Update last_encoder_val for the next iteration
            last_encoder_val = encoder_val;
            // --- END FIX ---
            
            let slider_val = buf[10];
            if slider_val != 0 {
                println!("Slider: {}", slider_val);
                let cnt = (slider_val as i32 - 1 + 5) * 25 / 200 - 1;
                for i in 0..25 {
                    let b = match cnt - i {
                        0 => Brightness::Normal,
                        1..=25 => Brightness::Dim,
                        _ => Brightness::Off,
                    };
                    lights.set_slider(i as usize, b);
                }
                changed_lights = true;
            }
        } else if buf[0] == 0x02 {
            // pad mode
            for i in (1..buf.len()).step_by(3) {
                let idx = buf[i];
                let evt = buf[i + 1] & 0xf0;
                let val = ((buf[i + 1] as u16 & 0x0f) << 8) + buf[i + 2] as u16;
                if i > 1 && idx == 0 && evt == 0 && val == 0 {
                    break;
                }
                let pad_evt: PadEventType = num::FromPrimitive::from_u8(evt).unwrap();
                // if evt != PadEventType::Aftertouch {
                println!("Pad {}: {:?} @ {}", idx, pad_evt, val);
                // }
                let (_, prev_b) = lights.get_pad(idx as usize);
                let b = match pad_evt {
                    PadEventType::NoteOn | PadEventType::PressOn => Brightness::Normal,
                    PadEventType::NoteOff | PadEventType::PressOff => Brightness::Off,
                    PadEventType::Aftertouch => {
                        if val > 0 {
                            Brightness::Normal
                        } else {
                            Brightness::Off
                        }
                    }
                    #[allow(unreachable_patterns)]
                    _ => prev_b,
                };
                if prev_b != b {
                    lights.set_pad(idx as usize, PadColors::Blue, b);
                    changed_lights = true;
                }

                let note = settings.notemaps[idx as usize];
                let mut velocity = (val >> 5) as u8;
                if val > 0 && velocity == 0 {
                    velocity = 1;
                }

                let event = match pad_evt {
                    PadEventType::NoteOn | PadEventType::PressOn => Some(MidiMessage::NoteOn {
                        key: note.into(),
                        vel: velocity.into(),
                    }),
                    PadEventType::NoteOff | PadEventType::PressOff => Some(MidiMessage::NoteOff {
                        key: note.into(),
                        vel: velocity.into(),
                    }),
                    _ => None,
                };

                if let Some(evt) = event {
                    let l_ev = LiveEvent::Midi {
                        channel: 0.into(),
                        message: evt,
                    };
                    let mut buf = Vec::new();
                    l_ev.write(&mut buf).unwrap();
                    port.send(&buf[..]).unwrap()
                }
            }
        }
        if changed_lights {
            lights.write(device)?;
        }
    }
}