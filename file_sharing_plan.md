# File sharing plan: SMB and FTP server in Thimsy

Status: planned, not started. MiXplorer is no longer shipped (removed in test 22). Its FTP/SMB server was the only reason left to ship a file manager, because Nano's File Explorer covers everything else. This plan replaces that server with one built into Thimsy.

## Goal

Let a PC, Mac or phone on the same network read and write the console's storage (internal and microSD) without cables or extra apps, so ROMs, BIOS files, saves and screenshots can be copied over Wi-Fi.

- SMB first: it's built into Windows, macOS, Linux file managers, iOS Files and Android file managers.
- FTP second: for older or simpler clients and scripted transfers.
- Off by default. Turned on from Nano. Nothing runs while it's off (see [no needless subprocesses]).

## User experience

**Settings > Network > File Sharing** in Nano:

| Row | Behaviour |
|---|---|
| File Sharing (SMB) | On/off switch. |
| FTP | On/off switch, independent of SMB. |
| User name | Default `thimsy`. |
| Password | Must be set before either switch can be turned on. Nano offers a generated 8-character password to accept or edit. |
| Address | Read-only: `\\thimsy.local\` and `\\192.168.x.x\` for SMB, `ftp://192.168.x.x:2121` for FTP. Refreshes when Wi-Fi changes. |
| Shared folders | Internal storage (`/storage/emulated/0`) and each microSD card, each as its own share (`Internal`, `SDCARD`). Each can be switched off. |
| Read only | Off by default. |

- While a server is on, the status bar shows a small share icon.
- The row says "Wi-Fi stays awake while File Sharing is on."
- With no Wi-Fi connection, the switches stay usable, but the Address row says "Connect to Wi-Fi to use File Sharing".
- Controller and touch both work, like every Nano screen. The password uses the existing on-screen keyboard.
- The Quick Menu gets no entry. This is a settings feature.

## Architecture

```
Nano (settings UI) --setprop--> init --starts/stops--> thimsy-fileshare (daemon, own SELinux domain)
                                                         |- SMB server (libsmb2 server API, port 445)
                                                         |- FTP server (own code, port 2121)
                                                         |- mDNS responder (thimsy.local, _smb._tcp, _ftp._tcp)
                                                         \- WS-Discovery responder (phase 3)
```

- One daemon, `/system/bin/thimsy-fileshare`, under `frameworks/base/cmds/thimsy-fileshare/`. Init starts it only while `persist.thimsy.share.smb` or `persist.thimsy.share.ftp` is `1`, the same pattern as `gammaos-sharefs.rc`:
  ```
  service thimsy-fileshare /system/bin/thimsy-fileshare
      class late_start
      user root
      group root media_rw inet
      capabilities NET_BIND_SERVICE DAC_OVERRIDE CHOWN FOWNER
      seclabel u:r:thimsy_fileshare:s0
      gentle_kill
      disabled
  on property:persist.thimsy.share.smb=1 && property:sys.boot_completed=1
      start thimsy-fileshare
  ```
  The daemon re-reads its properties on `SIGHUP`. Nano restarts the service when settings change. With both switches off, init stops it.
  - Open question: drop root after binding (setuid to `media_rw` with supplementary groups) if the capability set allows. Check during the spike.
- Nano never runs or talks to the daemon directly. It only sets properties and writes the config file, so the UI keeps no sockets and the daemon can't stall the UI.
- **Config:** `/data/misc/thimsy-fileshare/config` (0600, root), holding the user name, password, share switches and read-only flag. The password is stored there, not in a property: properties are readable by every app.
  - Nano runs as root (`gammaos-nano.rc`), so it writes the file directly: to a temp file, `fsync`, then `rename`. Once the daemon's SELinux domain exists, give Nano's domain write access to the config dir's type.
- **Files are served from the lower filesystem:** `/data/media/0` and `/mnt/media_rw/<uuid>`, not the FUSE `/storage` views. FUSE costs throughput, and the daemon needs no per-app permissions. New files are created `media_rw:media_rw` 0660 (directories 0770) so MediaProvider and apps see them.
  - After a write session ends (no clients for 10 s), the daemon asks MediaProvider to rescan the touched directories: `content call --uri content://media/none --method scan_volume` or `am broadcast` with a scan intent. Confirm which works on 14.

## SMB server

**Library:** `external/libsmb2`, already in the tree for the Network Shares client, LGPL-2.1. Its server API is in `include/smb2/libsmb2.h`:
- `struct smb2_server` and `smb2_server_request_handlers`;
- `smb2_bind_and_listen`, `smb2_serve_port`;
- `examples/smb2-server-sync.c` is a ~500-line working example.

**We write the handlers:**
- `tree_connect`: map the share name to its root and refuse unknown shares.
- `create` / `close`: open files and directories under the share root. Resolve every path with `openat2(RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS)` relative to the share root fd, so a client can never escape the share, even through symlinks.
- `read` / `write`: `pread` / `pwrite`. Writes are refused when read-only is set.
- `query_directory`: the info classes Windows and macOS actually use (`FileIdBothDirectoryInformation`, `FileBothDirectoryInformation`, `FileNamesInformation`), with wildcard filtering and resume.
- `query_info` / `set_info`:
  - basic, standard and network-open info;
  - end-of-file (truncate), rename (`renameat2`), delete-on-close;
  - filesystem size info from `statvfs`, so Explorer shows free space.
- `change_notify`: phase 2, via inotify, so Explorer windows refresh by themselves. Until then, return `STATUS_NOT_SUPPORTED`; clients fall back to refreshing.
- `ioctl`: `FSCTL_VALIDATE_NEGOTIATE_INFO` is required by Windows (the library has support). Everything else returns `STATUS_NOT_SUPPORTED`.
- `lock` / oplock / lease: grant no oplocks or leases, and reply to byte-range locks with success without enforcing them. A single-user handheld doesn't need them, and clients work without oplocks.

**Authentication:**
- NTLMv2 with the configured user and password. The `authorize_user` handler sets the expected password on the context, and libsmb2's NTLMSSP code verifies it.
- `allow_anonymous = 0`. No guest access: Windows 11 24H2 blocks guest logons by default anyway.

**Signing and encryption:**
- `signing_enabled = 1`. Windows 11 24H2 requires signing for outbound SMB by default, so this is mandatory.
- Offer SMB 3.x encryption if the library's server side supports it cleanly. Otherwise, signing over SMB 2.1/3.0 is enough for a home network.

**Dialects:** SMB 2.1 and 3.0/3.0.2 (and 3.1.1 if supported, which needs pre-auth integrity). Never SMB1.

**Concurrency:** one thread per client connection (`smb2_serve_port` callback), at most 4 clients. The library's sync server loop is per connection.

**Throughput target:** saturate the RG Cube's Wi-Fi (about 20–40 MB/s on 5 GHz). Use large max read/write sizes (1 MB) and avoid per-request allocations.

### Compatibility spike (do this first, before building the full UI)

Build the example server against `/data/media/0` on the device, then test the operations below.

**Clients:**
- Windows 11 24H2, with signing required and guest blocked;
- Windows 10;
- macOS Finder (Connect to Server, `smb://`);
- Linux (GNOME Files / `smbclient` / `mount.cifs`);
- iOS Files;
- an Android file manager.

**Operations:**
- browse;
- copy 1 GB in and out;
- copy 10,000 small files;
- rename, delete, make folder;
- free space shown;
- non-ASCII file names (Japanese ROM names);
- files over 4 GB;
- a client disconnecting mid-copy.

If libsmb2's server fails Windows or macOS in ways that are hard to fix, the fallbacks are:
1. fix it upstream;
2. ksmbd, the in-kernel server. That needs a kernel module we can't build for the vendor kernel, so it's unlikely;
3. a cross-compiled Samba `smbd`. GPL-3 and about 30 MB, so last resort.

## FTP server

- Our own small server inside the same daemon, about 1,000 lines. No suitable C/C++ FTP server library is in the tree.
- Port 2121, because 21 is often blocked or needs special handling on clients. It's configurable.
- **Commands:** USER, PASS, SYST, FEAT, OPTS UTF8, PWD/CWD/CDUP, TYPE, PASV/EPSV, LIST/MLSD/MLST/NLST, SIZE, MDTM, RETR (with REST), STOR, APPE, DELE, MKD, RMD, RNFR/RNTO, QUIT. No active mode (PORT): it breaks behind NAT and isn't needed on a LAN.
- The same path resolution (`openat2` beneath the share root) and the same credentials as SMB.
- `/` lists the enabled shares as folders.
- **FTPS (explicit, AUTH TLS) in phase 3, via BoringSSL.** It needs a self-signed certificate generated on first start. Plain FTP sends the password unencrypted, so the settings row says so: "FTP sends your password unencrypted. Use it only on networks you trust."
- One thread per control connection, at most 4.

## Discovery

- **mDNS (phase 1):** the daemon answers on 224.0.0.251:5353 for:
  - `thimsy.local` (A/AAAA records), so `\\thimsy.local` works on Windows 10+ and macOS;
  - `_smb._tcp`, so the console appears in the Finder sidebar and in Linux file managers;
  - `_ftp._tcp`.

  A minimal responder (answer queries for our names, announce at start, goodbye at stop) is about 300 lines.
  - Android 14's own mDNS (`NsdManager`, in the Tethering module) needs a Java caller. A self-contained responder keeps Nano native-only.
  - Check for conflicts with the system mDNS stack on 5353 (use `SO_REUSEADDR`/`SO_REUSEPORT`, and answer only our own names).
  - The host name defaults to `thimsy`. Add `-2`, `-3` on conflict (probe first, as RFC 6762 requires).
- **WS-Discovery (phase 3):** UDP 3702 multicast plus an HTTP metadata endpoint on 5357. This is what makes the console appear in Windows Explorer's Network folder, the way `wsdd` does on Linux, and it's about 400 lines. Until then, Windows users type the address shown in Nano.
- **No NetBIOS:** it's obsolete and SMB1-era.

## Security

- **Off by default.** It stays off across factory resets. Nothing listens unless the user turns it on.
- **A password is required.** There's no anonymous or guest mode.
- **Wi-Fi only:** the daemon binds to the Wi-Fi interface's address, and rebinds when the network changes. It doesn't serve on mobile hotspot clients unless the user is hosting the hotspot, in which case serving on it is fine.
- **Network type:** Android has no Windows-style "public network" flag. The settings row explains the risk instead, and the share icon in the status bar keeps it visible.
- **Failed logins:** after 5 failures from one address, refuse that address for 60 s.
- **Paths:** everything is resolved beneath the share root with `openat2` (no symlink following, no `..`). SELinux confines the daemon further: the new domain `thimsy_fileshare` may access only `media_rw_data_file`, `mnt_media_rw_file`/vfat/exfat, its own config dir and sockets.
- **Credentials:** the password lives only in the 0600 config file. It's never written to logs or properties.

## Power and lifecycle

- **Wi-Fi:** while a server is on, the daemon holds a Wi-Fi high-performance lock, through a tiny privileged call or by setting the Wi-Fi power-save property while running, so transfers don't stall when the screen dims.
- **No auto-off:** the server stays on until the user switches it off, a deliberate choice for long copies.
- **While a game runs:**
  - The server keeps running at background priority (`SCHED_IDLE` for transfer threads), so a copy can't stutter a game.
  - Thimsy's game-launch app stopping must not stop this daemon. It isn't an app, so it's unaffected. Confirm that the overlay's force-stop paths don't touch it.
- **Sleep:** the server stops serving while the screen is off in deep sleep. Clients reconnect afterwards, because SMB and FTP clients retry.

## Phases

1. **Spike:**
   - libsmb2 server example on the device;
   - the compatibility matrix above;
   - a go/no-go decision on libsmb2.
2. **SMB MVP:**
   - daemon and rc, SELinux domain and config file;
   - share roots;
   - all handlers except `change_notify`;
   - mDNS;
   - the Nano settings page, with translations.
3. **FTP:** plain FTP with the same config, plus `_ftp._tcp`.
4. **Polish:**
   - `change_notify` via inotify;
   - WS-Discovery;
   - FTPS;
   - SMB3 encryption;
   - MediaProvider rescans;
   - the status-bar icon.

## Testing

- **Unit tests (host):** path resolution (escape attempts, symlinks, Unicode), FTP command parser, mDNS packet encode/decode.
- **On device:** the full compatibility matrix for each release candidate.
- **Stress:** a 50 GB copy with the card nearly full (ENOSPC is reported as `STATUS_DISK_FULL` / FTP 552, and the partial file is deleted). Also pulling the microSD card mid-transfer: the share disappears, and the daemon must not crash.
- **Game impact:** a 1 GB copy during a PS2 game in ARMSX2, with no frame drops beyond baseline.

## Licences

- libsmb2: LGPL-2.1, linked dynamically as today (`libsmb2.so` is already shipped). Credit it in NOTICE.
- FTP server, mDNS and WS-Discovery: our own code, under the project licence.
- BoringSSL (FTPS): already in AOSP.
