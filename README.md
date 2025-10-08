# Maschine Mikro MK3 Linux Driver +
Userspace for linux to work with the Native Instruments Maschine Mikro MK3, aiming to complete control of the functionalities of the interface.

Forked from: [https://github.com/r00tman/maschine-mikro-mk3-driver/tree/main/crates]


## Getting Started

Let's install dependencies first:
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

Then we can proceed with the repo:

```shell
git clone https://github.com/davesuonabene/MaschineMikroMk3LinuxDriverPlus.git; cd MaschineMikroMk3LinuxDriverPlus
sudo cp 98-maschine.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger
cargo run --release example_config.toml
```

## Roadmap

What worked already (as the original from @r00tman):
 - Pads,
 - Buttons,
 - Encoder,
 - Slider,
 - LEDs,
 - Screen.

What I added:
 - OSC support for button presses
 - Selectable Button Modes (trigger, toggle)