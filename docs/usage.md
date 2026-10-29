# Usage

XP Music imitates Windows Media Player 9: a side bar with the tasks, the content in the center and the controls at the bottom left.

![XP Music library](library.jpg)

## Views

| Task | What it shows |
| --- | --- |
| Now Playing | The visualization (or the cover art) and the current playlist. |
| Discover | The YouTube Music home: recommendations, new releases and playlists. |
| Library | Your playlists, songs, albums and artists. |
| Liked | The songs you have marked with "Like". |
| Recent | Your listening history. |
| Search | Songs, albums, artists and playlists. |
| Options | Theme, language, audio, keyboard, account and visualization. |

The drop-down at the top right shows the playlist that is playing and lets you switch to another one, turn on shuffle, or turn on radio (automatic playback when the queue ends).

## My music

XP Music also plays music stored on your computer, with or without a YouTube Music account. In the Library, open **My music** and press **Add folder…** (or use the **My music** tab in Options). Subfolders are included, and the songs are listed by song, album and artist using their tags and cover art.

Supported formats: MP3, M4A/AAC (including ALAC), FLAC, Ogg Vorbis, WAV and AIFF. Opus files are not supported yet. The library is rescanned at startup and whenever you press **Rescan**; only new or changed files are read again.

Local songs and YouTube songs can share the same queue. Likes, playlists, radio and the listening history are YouTube Music features, so they are not available for local songs.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| Ctrl+P | Play / Pause |
| Ctrl+S | Stop |
| Ctrl+B | Previous |
| Ctrl+F | Next |
| Ctrl+H | Shuffle |
| Ctrl+T | Repeat |
| F8 | Mute |
| F9 / F10 | Volume down / up |
| Ctrl+E | Search |
| Ctrl+1 | Full mode |
| Ctrl+2 | Compact mode |

The keyboard's media keys always work. In **Options → Keyboard** you can define global shortcuts that work even when XP Music is in the background.

## Visualizations

In "Now Playing", the ◀ ▶ arrows (or a click on the stage) switch the visualization:

- Ambience: Water
- Alchemy: Plasma
- Alchemy: Spiral
- Ambience: Petals
- Battery: Kaleidoscope
- Bars and Waves: Bars
- Bars and Waves: Oscilloscope
- Album art

![Now Playing with a visualization](now-playing.jpg)

## Window modes

- **Full mode** (Ctrl+1): the normal window.
- **Compact mode** (Ctrl+2): just the controls.
- **Skin mode**: uses a Windows Media Player skin. See [Skins](skins.md).

## Options

In **Options** (or **Tools → Options…**) you can set:

- **Player:** color scheme (Blue, Olive green or Silver), light or dark appearance (follows the system by default), language (automatic by default, or English / Español), minimize to the tray, start minimized or when you sign in, notifications, system sounds, and pause when the computer is locked.
- **Keyboard:** global shortcuts.
- **Audio:** output device, even out the volume between songs, and record plays in the YouTube Music history.
- **Account:** the connected account; you sign out from here.
- **Visualization:** the one shown in "Now Playing".
- **About:** version, components (yt-dlp, Deno), updates and the logs folder.
