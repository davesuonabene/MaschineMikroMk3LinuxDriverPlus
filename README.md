# Maschine Mikro MK3 Linux Driver +
I got very upset that Native Instruments never got the time to make their Maschine useful for Linux users.
This work solves that, hacking the Maschine Mikro MK3 to work with Linux with extended control over the interface capabilities for uses with DAWs or any other OSC capable software.

Forked from: [https://github.com/r00tman/maschine-mikro-mk3-driver/tree/main/crates]

## Overview

- Complete Hardware Exploitation
- Open Sound Control (OSC) I/O
- Custom MIDI mapping for all the buttons and pads (with modes, MIDI types and button groups)

**This may be the start of a series of drivers written to unlock the full potential of great controllers such as the Maschine, made useless by the ignorance of companies that would let their instruments fall into the dark instead of opening up the possibility for us to use what we bought.

## Prerequisites

- Debian/Ubuntu:
  ```
  sudo apt install build-essential pkg-config libasound2-dev libjack-dev libusb-1.0-0-dev libudev-dev
  ```
- Fedora/RHEL:
  ```
  sudo dnf install @development-tools alsa-lib-devel jack-audio-connection-kit-devel libusb-devel systemd-devel
  ```
- Arch Linux:
  ```
  sudo pacman -S base-devel alsa-lib pipewire-jack libusb systemd-libs  # (or `jack2` instead of `pipewire-jack`)
  ``` 

## Installation and Build

```shell
git clone https://github.com/davesuonabene/MaschineMikroMk3LinuxDriverPlus.git; cd MaschineMikroMk3LinuxDriverPlus
sudo cp 98-maschine.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
cargo run --release example_config.toml
```

## Roadmap

What worked already (as the original from @r00tman):
input from {
 - Pads [with midi output],
 - Buttons,
 - Encoder,
 - Slider,
 - LEDs,
 - Screen.
}

What I added:
 - OSC Output [buttons, slider]
 - OSC Input (for led control)
 - Custom Button Modes (trigger, toggle)
 - Custom MIDI CC Out for buttons
