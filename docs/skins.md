# Windows Media Player skins

XP Music reads the original WMP skins (`.wmz`). It does not include any: copy the ones you have into the skins folder and press **Refresh**.

## Where they go

Open **View → Skin Chooser… → Open folder**. It is:

| System | Folder |
| --- | --- |
| Windows | `%APPDATA%\xp-music\skins` |
| Linux | `~/.config/xp-music/skins` (or `$XDG_CONFIG_HOME/xp-music/skins`) |

You can organize them in subfolders.

## Going back to the normal look

- **Ctrl+1**.
- Right-click the skin → "Back to full mode".
- From the tray icon.

## Limitations

- Each skin's scripts run isolated, with no access to your account or your files.
- Secondary windows (equalizer, separate playlist) do not open yet.
- On Linux, click-through on the transparent areas does not work.
