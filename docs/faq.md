# FAQ

## Is it official?

No. XP Music is not affiliated with, sponsored by or endorsed by Google, YouTube or Microsoft. "YouTube Music", "Windows Media Player" and "Windows XP" are trademarks of their respective owners. The app uses your own account for what you would do from the browser.

## Windows says the app may be dangerous

The installer is signed, but not with a commercial certificate, so SmartScreen warns you the first few times. Click "More info" → "Run anyway".

## Where is my session stored?

On Windows, encrypted with DPAPI (tied to your user). On Linux, in the Secret Service (GNOME Keyring, KeePassXC); if it is not available, the session is not stored in plain text.

## On Linux I cannot sign in or the account is forgotten

Check that you have an unlocked Secret Service (GNOME Keyring, or KeePassXC with that integration enabled).

## On Linux, closing the window quits the app

This is intentional: on many desktops (such as GNOME without the AppIndicator extension) the tray icon is not visible. If yours shows it, turn on "When closing, keep running in the notification area" in Options.

## A song does not play

XP Music gets the audio with [yt-dlp](https://github.com/yt-dlp/yt-dlp). If YouTube changes something, try **Options → About → Check for yt-dlp update**. If it still fails, open the logs folder from the same place and attach the file when you open an *issue*.

## How do I change the language?

XP Music follows your system language (Spanish if it is Spanish, English otherwise). To change it, go to **Options → Player → Language**.

## Where do I report a bug?

In the [GitHub issues](https://github.com/Iznardo/xp-music/issues).
