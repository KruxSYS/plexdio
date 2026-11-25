Plexdio @VERSION@ (portable)
============================

A small, native Windows 10/11 x64 music player for Plex, by KruxSYS.
Website: https://plexdio.kruxsys.com

Run plexdio.exe from this folder. Nothing is installed.

The file "portable" next to plexdio.exe makes this a portable copy: settings, the library
cache, logs and downloads are kept in the Data folder next to it. Move or copy the whole
folder to take them along; delete it to remove Plexdio. Your Plex sign-in is kept in the
Windows Credential Manager (search for "Plexdio" there to remove it).

Updates: Plexdio checks plexdio.kruxsys.com once a day (Settings > About). "Update now"
downloads the new zip, checks its SHA-256 and replaces plexdio.exe in this folder; the
Data folder and the portable marker stay as they are.

The files are not code-signed, so Windows SmartScreen may warn the first time you run
plexdio.exe ("More info" > "Run anyway").

Prefer an installer (Start menu entry, uninstaller)? Get Plexdio-@VERSION@-win-x64-setup.exe
from https://plexdio.kruxsys.com/download.

Licence: MIT, see LICENSE.txt. (c) 2026 KruxSYS.
