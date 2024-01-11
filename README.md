# discord-updater

Because Discord Canary updates so often and I can't be arsed to continuously check the AUR, I decided to write this small binary script which automatically downloads the latest version (for Linux currently) and extracts it to the home directory. The only current argument is `--download-dir` if you wish to modify the output directory.

I don't think the script will really be discovered or will be useful to anyone else, but I generalized it enough so that it doesn't break when switching to different Linux environments atleast. macOS support could be added if I wasn't lazy enough to choose the dmg download link in the code rather than the linux zip, but I don't even know how most of the Discord API works anyway.
