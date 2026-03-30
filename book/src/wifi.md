# Setting Up WiFi

The dashboard needs WiFi to fetch your Strava data. There are two ways to
configure it.

## Option 1: During SD card flashing (easiest)

If you're using the Raspberry Pi Imager (see [Preparing the SD Card](./sd-card.md)),
you can enter your WiFi credentials directly in the imager settings. This is
the easiest approach and works out of the box.

## Option 2: Adding a NetworkManager connection file

The PhotoPainter's pre-flashed OS uses **NetworkManager** to manage network
connections. You can add a WiFi configuration file directly to the SD card.

### On your computer

If you have the project's source code and the `just` command runner installed,
you can generate the file automatically:

```bash
just wifi "YourWiFiName" "YourWiFiPassword"
```

This creates a file at `tmp/YourWiFiName.nmconnection`.

### Creating the file manually

If you don't have `just`, create a file called `YourWiFiName.nmconnection`
with this content (replace the values in caps):

```ini
[connection]
id=YOUR_WIFI_NAME
uuid=any-unique-string-here
type=wifi
autoconnect=true

[wifi]
mode=infrastructure
ssid=YOUR_WIFI_NAME

[wifi-security]
key-mgmt=wpa-psk
psk=YOUR_WIFI_PASSWORD

[ipv4]
method=auto

[ipv6]
method=auto
```

### Copying the file to the SD card or Pi

**Before first boot (SD card):**

Mount the SD card's root partition and copy the file:

```bash
sudo cp YourWiFiName.nmconnection \
  /path/to/sdcard/etc/NetworkManager/system-connections/
sudo chmod 600 \
  /path/to/sdcard/etc/NetworkManager/system-connections/YourWiFiName.nmconnection
```

**After first boot (over SSH):**

If the Pi is already running and you can reach it (e.g., via ethernet or
another WiFi network):

```bash
scp YourWiFiName.nmconnection pi@photopainter.local:~/
ssh pi@photopainter.local
sudo mv ~/YourWiFiName.nmconnection /etc/NetworkManager/system-connections/
sudo chmod 600 /etc/NetworkManager/system-connections/YourWiFiName.nmconnection
sudo nmcli connection reload
sudo nmcli connection up "YourWiFiName"
```

## Verifying the connection

Once the Pi is connected to WiFi, check with:

```bash
ssh pi@photopainter.local
ping -c 3 8.8.8.8
```

If pings succeed, WiFi is working and you're ready to continue.
